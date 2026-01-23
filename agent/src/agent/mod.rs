//! Agent module - LLM with tool-calling capabilities
//!
//! This implements the "tool-using agent loop" where:
//! 1. User sends a message
//! 2. LLM receives the message along with available tools
//! 3. LLM decides whether to call tools or respond directly
//! 4. If tools are called, results are fed back to LLM
//! 5. Loop continues until LLM responds without tool calls

use std::time::Instant;

use anyhow::{Context, Result};

use crate::mcp::{McpClientPool, McpTool};

// Modular parser system for handling different tool call formats
pub mod parsers;
use parsers::{ToolCall, ToolCallParserRegistry};

// Agent event emission for real-time visibility
pub mod events;
pub use events::{AgentEvent, AgentEventSender, EventReceiver, EventSender, event_channel};

// Direct HTTP API types for Ollama
mod types;
pub use types::DirectMessage;
use types::{DirectChatRequest, DirectChatResponse, DirectTool};

// Tool-related utilities
mod tools;
use tools::mcp_tools_to_direct;

/// Maximum number of tool-calling iterations to prevent infinite loops
const MAX_ITERATIONS: usize = 10;


/// An agent that can use tools via MCP
pub struct Agent {
    ollama_url: String,
    http_client: reqwest::Client,
    model: String,
    mcp_pool: McpClientPool,
    system_prompt: Option<String>,
    history: Vec<DirectMessage>,
    parser_registry: ToolCallParserRegistry,
    verbose: bool,
    event_sender: AgentEventSender,
}

impl Agent {
    /// Create a new agent
    pub fn new(ollama_url: &str, model: &str, mcp_pool: McpClientPool) -> Self {
        let url = url::Url::parse(ollama_url).unwrap_or_else(|_| {
            url::Url::parse("http://localhost:11434").unwrap()
        });

        let host = url.host_str().unwrap_or("localhost").to_string();
        let port = url.port().unwrap_or(11434);
        let base_url = format!("http://{}:{}", host, port);

        Self {
            ollama_url: base_url,
            http_client: reqwest::Client::new(),
            model: model.to_string(),
            mcp_pool,
            system_prompt: None,
            history: Vec::new(),
            parser_registry: ToolCallParserRegistry::new(),
            verbose: false,
            event_sender: AgentEventSender::none(),
        }
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = Some(prompt.to_string());
        self
    }

    /// Enable verbose timing output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set event sender for real-time event visibility
    pub fn with_event_sender(mut self, sender: EventSender) -> Self {
        self.event_sender = AgentEventSender::new(sender);
        self
    }

    /// Set event sender dynamically
    pub fn set_event_sender(&mut self, sender: Option<EventSender>) {
        self.event_sender = match sender {
            Some(s) => AgentEventSender::new(s),
            None => AgentEventSender::none(),
        };
    }

    /// Update the system prompt dynamically
    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.system_prompt = prompt;
    }

    /// Get current conversation history (for session storage)
    pub fn get_history(&self) -> Vec<DirectMessage> {
        self.history.clone()
    }

    /// Set conversation history (for session restoration)
    pub fn set_history(&mut self, history: Vec<DirectMessage>) {
        self.history = history;
    }

    /// Get the current model name
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Switch to a different model at runtime
    pub fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    /// Get the Ollama URL
    pub fn ollama_url(&self) -> &str {
        &self.ollama_url
    }

    /// Get the current system prompt
    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    /// Run a single message through the agent, handling tool calls
    /// Uses direct HTTP to Ollama API (bypasses ollama-rs for better tool calling support)
    /// NOTE: With many tools (>20), smaller models may not use tool calling reliably.
    /// Use chat_with_servers() to filter to specific MCP servers.
    pub async fn chat(&mut self, user_message: &str) -> Result<String> {
        // Get available tools from MCP
        let mcp_tools = self.mcp_pool.list_all_tools().await?;
        let tools = mcp_tools_to_direct(&mcp_tools);

        tracing::info!("Agent has {} tools available", tools.len());

        // Warn if too many tools for smaller models
        if tools.len() > 20 {
            tracing::warn!(
                "Many tools ({}) may cause issues with smaller models. Consider using chat_with_servers() to filter.",
                tools.len()
            );
        }

        self.chat_with_tools(user_message, tools).await
    }

    /// Clear conversation history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Get available tool names
    pub async fn tool_names(&mut self) -> Result<Vec<String>> {
        let tools = self.mcp_pool.list_all_tools().await?;
        Ok(tools.into_iter().map(|t| t.name).collect())
    }

    /// Get list of available MCP server names
    pub async fn server_names(&mut self) -> Result<Vec<String>> {
        let tools = self.mcp_pool.list_all_tools().await?;
        let mut servers: Vec<String> = tools.iter().map(|t| t.server.clone()).collect();
        servers.sort();
        servers.dedup();
        Ok(servers)
    }

    /// Get tools for a specific server
    pub async fn tools_for_server(&mut self, server_name: &str) -> Result<Vec<McpTool>> {
        let tools = self.mcp_pool.list_all_tools().await?;
        let filtered: Vec<_> = tools
            .into_iter()
            .filter(|t| t.server == server_name)
            .collect();
        Ok(filtered)
    }

    /// Run a chat with tools filtered to specific MCP servers
    /// This is useful when you have many tools but only need a subset
    pub async fn chat_with_servers(&mut self, user_message: &str, servers: &[&str]) -> Result<String> {
        // Get tools filtered by server
        let all_tools = self.mcp_pool.list_all_tools().await?;
        let filtered_tools: Vec<_> = all_tools
            .into_iter()
            .filter(|t| servers.contains(&t.server.as_str()))
            .collect();

        let tools = mcp_tools_to_direct(&filtered_tools);
        tracing::info!("Agent has {} tools available (filtered from servers: {:?})", tools.len(), servers);

        self.chat_with_tools(user_message, tools).await
    }

    /// Internal method to run chat with a specific set of tools
    async fn chat_with_tools(&mut self, user_message: &str, tools: Vec<DirectTool>) -> Result<String> {
        let total_start = Instant::now();

        // Emit processing start event
        self.event_sender.processing_start(user_message);

        // Build initial messages
        let mut messages: Vec<DirectMessage> = Vec::new();

        // Add system prompt if set
        if let Some(ref system) = self.system_prompt {
            messages.push(DirectMessage {
                role: "system".to_string(),
                content: system.clone(),
                tool_calls: None,
            });
        }

        // Add history
        messages.extend(self.history.clone());

        // Add new user message
        messages.push(DirectMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
            tool_calls: None,
        });

        // Tool-calling loop
        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                tracing::warn!("Agent reached max iterations ({}), stopping", MAX_ITERATIONS);
                break;
            }

            tracing::debug!("Agent iteration {}", iterations);

            // Emit iteration event (tool_calls count from previous iteration, 0 for first)
            // We'll emit another event after we know the tool count
            self.event_sender.iteration(iterations, 0);

            // Create direct HTTP request
            let request = DirectChatRequest {
                model: self.model.clone(),
                messages: messages.clone(),
                tools: tools.clone(),
                stream: false,
            };

            // Log request info
            tracing::info!("=== OLLAMA REQUEST (Direct HTTP) ===");
            tracing::info!("Model: {}", self.model);
            tracing::info!("Messages count: {}", messages.len());
            tracing::info!("Tools count: {}", tools.len());

            // Verbose feedback before Ollama call
            if self.verbose {
                eprint!("[       ...] Waiting for {} ", self.model);
                if iterations == 1 {
                    eprintln!("({}msg, {}tools)", messages.len(), tools.len());
                } else {
                    eprintln!("(iteration {})", iterations);
                }
            }

            // Send direct HTTP request
            let url = format!("{}/api/chat", self.ollama_url);
            let ollama_start = Instant::now();
            let response = self
                .http_client
                .post(&url)
                .json(&request)
                .send()
                .await
                .context("Failed to send HTTP request to Ollama")?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!("Ollama API error {}: {}", status, body));
            }

            let raw_body = response.text().await.context("Failed to get response text")?;
            let response_body: DirectChatResponse = serde_json::from_str(&raw_body)
                .context("Failed to parse Ollama response")?;
            let ollama_elapsed = ollama_start.elapsed();

            let assistant_msg = response_body.message;

            // Log response
            tracing::info!("=== OLLAMA RESPONSE ===");
            tracing::info!("Content length: {}", assistant_msg.content.len());
            if !assistant_msg.content.is_empty() {
                tracing::info!("Content preview: {}", &assistant_msg.content[..assistant_msg.content.len().min(200)]);
            }
            tracing::info!("Tool calls count: {}", assistant_msg.tool_calls.len());

            // Verbose timing output
            if self.verbose {
                let tool_count = if assistant_msg.tool_calls.is_empty() {
                    "final".to_string()
                } else {
                    format!("{} tool call(s)", assistant_msg.tool_calls.len())
                };
                eprintln!("[{:>7}ms] Ollama response ({})", ollama_elapsed.as_millis(), tool_count);
            }
            for (i, tc) in assistant_msg.tool_calls.iter().enumerate() {
                tracing::info!("Tool call {}: {} args={:?}", i, tc.function.name, tc.function.arguments);
            }

            // Check if there are tool calls
            // First check the standard tool_calls array, then fallback to parsing content via registry
            let tool_calls = if !assistant_msg.tool_calls.is_empty() {
                assistant_msg.tool_calls.clone()
            } else if let Some((parsed_call, parser_name)) = self.parser_registry.parse(&assistant_msg.content) {
                // Fallback: model output tool call as JSON in content (common with qwen)
                tracing::info!(
                    "Parsed tool call from content using {}: {}",
                    parser_name,
                    parsed_call.function.name
                );
                vec![parsed_call]
            } else {
                vec![]
            };

            if tool_calls.is_empty() {
                // No tool calls - we're done
                tracing::info!("Agent responding without tool calls");

                let total_duration = total_start.elapsed();

                // Emit response complete event
                self.event_sender.response_complete(
                    &assistant_msg.content,
                    iterations,
                    total_duration,
                );

                if self.verbose {
                    eprintln!("[{:>7}ms] Total ({} iteration{})",
                        total_duration.as_millis(),
                        iterations,
                        if iterations == 1 { "" } else { "s" }
                    );
                }

                // Add to history
                self.history.push(DirectMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                    tool_calls: None,
                });
                self.history.push(DirectMessage {
                    role: "assistant".to_string(),
                    content: assistant_msg.content.clone(),
                    tool_calls: None,
                });

                return Ok(assistant_msg.content);
            }

            // Process tool calls
            tracing::info!(
                "Agent making {} tool call(s)",
                tool_calls.len()
            );

            // Add assistant message with tool calls to messages
            messages.push(DirectMessage {
                role: "assistant".to_string(),
                content: assistant_msg.content.clone(),
                tool_calls: Some(tool_calls.clone()),
            });

            // Execute each tool call and add results
            for tool_call in &tool_calls {
                // Emit tool start event
                self.event_sender.tool_start(
                    &tool_call.function.name,
                    &tool_call.function.arguments,
                );

                // Verbose feedback before tool call
                if self.verbose {
                    eprint!("[       ...] Calling {} ", tool_call.function.name);
                    // Show truncated args if available
                    let args_str = tool_call.function.arguments.to_string();
                    if args_str.len() > 60 {
                        eprintln!("({}...)", &args_str[..60]);
                    } else {
                        eprintln!("({})", args_str);
                    }
                }

                let tool_start = Instant::now();
                let (result, is_error) = match tools::execute_tool_call(&mut self.mcp_pool, tool_call).await {
                    Ok(r) => (r, false),
                    Err(e) => {
                        (format!("Error calling tool {}: {}", tool_call.function.name, e), true)
                    }
                };
                let tool_elapsed = tool_start.elapsed();

                // Emit tool complete event
                self.event_sender.tool_complete(
                    &tool_call.function.name,
                    &result,
                    tool_elapsed,
                    is_error,
                );

                if self.verbose {
                    eprintln!("[{:>7}ms] → {}", tool_elapsed.as_millis(), tool_call.function.name);
                }

                // Create tool response message
                messages.push(DirectMessage {
                    role: "tool".to_string(),
                    content: result,
                    tool_calls: None,
                });
            }
        }

        // Final timing summary
        let total_duration = total_start.elapsed();
        if self.verbose {
            eprintln!("[{:>7}ms] Total ({} iteration{})",
                total_duration.as_millis(),
                iterations,
                if iterations == 1 { "" } else { "s" }
            );
        }

        // Emit error event for max iterations
        let error_msg = format!(
            "Agent reached maximum iterations ({}) without completing",
            MAX_ITERATIONS
        );
        self.event_sender.error(&error_msg);

        // If we hit max iterations, return last assistant message
        Ok(error_msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_conversion() {
        let mcp_tool = McpTool {
            server: "test".to_string(),
            name: "get_weather".to_string(),
            description: Some("Get weather for a city".to_string()),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "city": {
                        "type": "string",
                        "description": "City name"
                    }
                },
                "required": ["city"]
            })),
        };

        let direct_tools = tools::mcp_tools_to_direct(&[mcp_tool]);

        assert_eq!(direct_tools.len(), 1);
        assert_eq!(direct_tools[0].function.name, "get_weather");
        assert_eq!(
            direct_tools[0].function.description,
            "Get weather for a city"
        );
    }
}
