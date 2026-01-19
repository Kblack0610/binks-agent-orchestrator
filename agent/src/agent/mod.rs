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
use ollama_rs::Ollama;
use serde::{Deserialize, Serialize};

use crate::mcp::{McpClientPool, McpTool};

// Modular parser system for handling different tool call formats
pub mod parsers;
use parsers::{ToolCall, ToolCallParserRegistry};

// Direct HTTP API types for Ollama (bypass ollama-rs for tool calls)
#[derive(Debug, Serialize)]
struct DirectChatRequest {
    model: String,
    messages: Vec<DirectMessage>,
    tools: Vec<DirectTool>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Serialize, Clone)]
struct DirectTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: DirectToolFunction,
}

#[derive(Debug, Serialize, Clone)]
struct DirectToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct DirectChatResponse {
    message: DirectResponseMessage,
}

#[derive(Debug, Deserialize)]
struct DirectResponseMessage {
    role: String,
    content: String,
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
}

// DirectToolCall and DirectToolCallFunction are now provided by parsers module
// as ToolCall and ToolCallFunction

/// Maximum number of tool-calling iterations to prevent infinite loops
const MAX_ITERATIONS: usize = 10;


/// An agent that can use tools via MCP
pub struct Agent {
    ollama: Ollama,
    ollama_url: String,
    http_client: reqwest::Client,
    model: String,
    mcp_pool: McpClientPool,
    system_prompt: Option<String>,
    history: Vec<DirectMessage>,
    parser_registry: ToolCallParserRegistry,
    verbose: bool,
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
            ollama: Ollama::new(format!("http://{}", host), port),
            ollama_url: base_url,
            http_client: reqwest::Client::new(),
            model: model.to_string(),
            mcp_pool,
            system_prompt: None,
            history: Vec::new(),
            parser_registry: ToolCallParserRegistry::new(),
            verbose: false,
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

    /// Clean up a JSON schema for Ollama compatibility
    /// Removes $schema, title, and other fields that confuse Ollama
    fn clean_schema_for_ollama(schema: &serde_json::Value) -> serde_json::Value {
        match schema {
            serde_json::Value::Object(obj) => {
                let mut cleaned = serde_json::Map::new();
                for (key, value) in obj {
                    // Skip fields that Ollama doesn't expect
                    if key == "$schema" || key == "title" || key == "additionalProperties" {
                        continue;
                    }
                    // Recursively clean nested objects (like properties)
                    cleaned.insert(key.clone(), Self::clean_schema_for_ollama(value));
                }
                serde_json::Value::Object(cleaned)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(Self::clean_schema_for_ollama).collect())
            }
            other => other.clone(),
        }
    }

    /// Convert MCP tools to direct API format
    fn mcp_tools_to_direct(tools: &[McpTool]) -> Vec<DirectTool> {
        tools
            .iter()
            .map(|tool| {
                let parameters = tool
                    .input_schema
                    .clone()
                    .map(|schema| Self::clean_schema_for_ollama(&schema))
                    .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));

                DirectTool {
                    tool_type: "function".to_string(),
                    function: DirectToolFunction {
                        name: tool.name.clone(),
                        description: tool.description.clone().unwrap_or_default(),
                        parameters,
                    },
                }
            })
            .collect()
    }

    /// Execute a tool call via MCP
    async fn execute_tool_call(&mut self, tool_call: &ToolCall) -> Result<String> {
        let name = &tool_call.function.name;
        let args = &tool_call.function.arguments;

        tracing::info!("Executing tool: {} with args: {:?}", name, args);

        // Call the tool via MCP
        let result = self
            .mcp_pool
            .call_tool(name, Some(args.clone()))
            .await
            .context(format!("Failed to call tool: {}", name))?;

        // Extract text content from result
        let mut output = String::new();
        for content in &result.content {
            match &content.raw {
                rmcp::model::RawContent::Text(text) => {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str(&text.text);
                }
                _ => {
                    // For non-text content, serialize it
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str(&format!("{:?}", content));
                }
            }
        }

        tracing::info!("Tool {} returned: {}...", name, &output[..output.len().min(100)]);

        Ok(output)
    }

    /// Run a single message through the agent, handling tool calls
    /// Uses direct HTTP to Ollama API (bypasses ollama-rs for better tool calling support)
    /// NOTE: With many tools (>20), smaller models may not use tool calling reliably.
    /// Use chat_with_servers() to filter to specific MCP servers.
    pub async fn chat(&mut self, user_message: &str) -> Result<String> {
        // Get available tools from MCP
        let mcp_tools = self.mcp_pool.list_all_tools().await?;
        let tools = Self::mcp_tools_to_direct(&mcp_tools);

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

    /// Run a chat with tools filtered to specific MCP servers
    /// This is useful when you have many tools but only need a subset
    pub async fn chat_with_servers(&mut self, user_message: &str, servers: &[&str]) -> Result<String> {
        // Get tools filtered by server
        let all_tools = self.mcp_pool.list_all_tools().await?;
        let filtered_tools: Vec<_> = all_tools
            .into_iter()
            .filter(|t| servers.contains(&t.server.as_str()))
            .collect();

        let tools = Self::mcp_tools_to_direct(&filtered_tools);
        tracing::info!("Agent has {} tools available (filtered from servers: {:?})", tools.len(), servers);

        self.chat_with_tools(user_message, tools).await
    }

    /// Internal method to run chat with a specific set of tools
    async fn chat_with_tools(&mut self, user_message: &str, tools: Vec<DirectTool>) -> Result<String> {
        let total_start = Instant::now();

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

                if self.verbose {
                    eprintln!("[{:>7}ms] Total ({} iteration{})",
                        total_start.elapsed().as_millis(),
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
                let tool_start = Instant::now();
                let result = match self.execute_tool_call(tool_call).await {
                    Ok(r) => r,
                    Err(e) => {
                        format!("Error calling tool {}: {}", tool_call.function.name, e)
                    }
                };
                let tool_elapsed = tool_start.elapsed();

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
        if self.verbose {
            eprintln!("[{:>7}ms] Total ({} iteration{})",
                total_start.elapsed().as_millis(),
                iterations,
                if iterations == 1 { "" } else { "s" }
            );
        }

        // If we hit max iterations, return last assistant message
        Ok("Agent reached maximum iterations without completing.".to_string())
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

        let direct_tools = Agent::mcp_tools_to_direct(&[mcp_tool]);

        assert_eq!(direct_tools.len(), 1);
        assert_eq!(direct_tools[0].function.name, "get_weather");
        assert_eq!(
            direct_tools[0].function.description,
            "Get weather for a city"
        );
    }
}
