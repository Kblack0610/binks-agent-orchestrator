//! Agent module - LLM with tool-calling capabilities
//!
//! This implements the "tool-using agent loop" where:
//! 1. User sends a message
//! 2. LLM receives the message along with available tools
//! 3. LLM decides whether to call tools or respond directly
//! 4. If tools are called, results are fed back to LLM
//! 5. Loop continues until LLM responds without tool calls

use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::mcp::{McpClientPool, McpTool};

// Modular parser system for handling different tool call formats
pub mod parsers;
use parsers::ToolCallParserRegistry;

// Model capability detection and classification
pub mod capabilities;
pub use capabilities::{
    detect_capabilities, strip_think_tags, FunctionCallFormat, ModelCapabilities,
    ModelCapabilityOverride,
};

// Agent event emission for real-time visibility
pub mod events;
pub use events::{event_channel, AgentEvent, AgentEventSender, EventReceiver, EventSender};

// Direct HTTP API types for Ollama
mod types;
pub use types::DirectMessage;
use types::{DirectChatRequest, DirectChatResponse, DirectTool};

// Tool-related utilities
mod tools;
use tools::mcp_tools_to_direct;

// MCP tool call metrics and observability
pub mod metrics;
use metrics::McpMetrics;

/// Default maximum iterations (used when config not provided)
const DEFAULT_MAX_ITERATIONS: usize = 10;
/// Default LLM timeout in seconds (5 minutes)
const DEFAULT_LLM_TIMEOUT_SECS: u64 = 300;
/// Default tool timeout in seconds (1 minute)
const DEFAULT_TOOL_TIMEOUT_SECS: u64 = 60;
/// Default max history messages
const DEFAULT_MAX_HISTORY_MESSAGES: usize = 100;

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
    // Stability configuration
    max_iterations: usize,
    llm_timeout: Duration,
    tool_timeout: Duration,
    max_history_messages: usize,
    // MCP observability
    mcp_metrics: McpMetrics,
    // Model capabilities (tool calling, thinking, etc.)
    capabilities: ModelCapabilities,
}

impl Agent {
    /// Create a new agent with default stability settings
    pub fn new(ollama_url: &str, model: &str, mcp_pool: McpClientPool) -> Self {
        Self::with_config(
            ollama_url,
            model,
            mcp_pool,
            DEFAULT_MAX_ITERATIONS,
            DEFAULT_LLM_TIMEOUT_SECS,
            DEFAULT_TOOL_TIMEOUT_SECS,
            DEFAULT_MAX_HISTORY_MESSAGES,
        )
    }

    /// Create a new agent with settings from AgentSectionConfig (.agent.toml)
    pub fn from_agent_config(
        ollama_url: &str,
        model: &str,
        mcp_pool: McpClientPool,
        config: &crate::config::AgentSectionConfig,
    ) -> Self {
        Self::with_config(
            ollama_url,
            model,
            mcp_pool,
            config.max_iterations,
            config.llm_timeout_secs,
            config.tool_timeout_secs,
            config.max_history_messages,
        )
    }

    /// Create a new agent with custom stability configuration
    pub fn with_config(
        ollama_url: &str,
        model: &str,
        mcp_pool: McpClientPool,
        max_iterations: usize,
        llm_timeout_secs: u64,
        tool_timeout_secs: u64,
        max_history_messages: usize,
    ) -> Self {
        let url = url::Url::parse(ollama_url)
            .unwrap_or_else(|_| url::Url::parse("http://localhost:11434").unwrap());

        let host = url.host_str().unwrap_or("localhost").to_string();
        let port = url.port().unwrap_or(11434);
        let base_url = format!("http://{}:{}", host, port);

        let llm_timeout = Duration::from_secs(llm_timeout_secs);

        // Create HTTP client with configured LLM timeout
        let http_client = reqwest::Client::builder()
            .timeout(llm_timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            ollama_url: base_url,
            http_client,
            model: model.to_string(),
            mcp_pool,
            system_prompt: None,
            history: Vec::new(),
            parser_registry: ToolCallParserRegistry::new(),
            verbose: false,
            event_sender: AgentEventSender::none(),
            max_iterations,
            llm_timeout,
            tool_timeout: Duration::from_secs(tool_timeout_secs),
            max_history_messages,
            mcp_metrics: McpMetrics::new(),
            capabilities: ModelCapabilities::default(),
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

    /// Set model capabilities (affects tool calling and response processing)
    pub fn with_capabilities(mut self, capabilities: ModelCapabilities) -> Self {
        self.capabilities = capabilities;

        // Reconfigure parser registry based on preferred format
        if self.capabilities.function_call_format != FunctionCallFormat::Native {
            self.parser_registry = ToolCallParserRegistry::with_preferred_format(
                self.capabilities.function_call_format,
            );
        }

        self
    }

    /// Get the current model capabilities
    pub fn capabilities(&self) -> &ModelCapabilities {
        &self.capabilities
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

    /// Get MCP tool call metrics
    pub fn mcp_metrics(&self) -> &metrics::McpMetrics {
        &self.mcp_metrics
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

    /// Prune conversation history to stay within max_history_messages limit
    /// Keeps most recent messages, discarding oldest ones first
    fn prune_history(&mut self) {
        if self.history.len() > self.max_history_messages {
            let excess = self.history.len() - self.max_history_messages;
            self.history.drain(0..excess);
            tracing::debug!(
                "Pruned {} messages from history (now {} messages)",
                excess,
                self.history.len()
            );
        }
    }

    /// Get current stability configuration
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    pub fn llm_timeout(&self) -> Duration {
        self.llm_timeout
    }

    pub fn tool_timeout(&self) -> Duration {
        self.tool_timeout
    }

    pub fn max_history_messages(&self) -> usize {
        self.max_history_messages
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
    pub async fn chat_with_servers(
        &mut self,
        user_message: &str,
        servers: &[&str],
    ) -> Result<String> {
        // Get tools filtered by server
        let all_tools = self.mcp_pool.list_all_tools().await?;
        let filtered_tools: Vec<_> = all_tools
            .into_iter()
            .filter(|t| servers.contains(&t.server.as_str()))
            .collect();

        let tools = mcp_tools_to_direct(&filtered_tools);
        tracing::info!(
            "Agent has {} tools available (filtered from servers: {:?})",
            tools.len(),
            servers
        );

        self.chat_with_tools(user_message, tools).await
    }

    /// Internal method to run chat with a specific set of tools
    async fn chat_with_tools(
        &mut self,
        user_message: &str,
        tools: Vec<DirectTool>,
    ) -> Result<String> {
        let total_start = Instant::now();

        // Emit processing start event
        self.event_sender.processing_start(user_message);

        // If model doesn't support tool calling, don't send tools
        // (empty Vec is omitted from JSON via skip_serializing_if)
        let effective_tools = if self.capabilities.tool_calling {
            tools
        } else {
            tracing::info!(
                "Model {} doesn't support tool calling, omitting tools",
                self.model
            );
            Vec::new()
        };

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
            if iterations > self.max_iterations {
                tracing::warn!(
                    "Agent reached max iterations ({}), stopping",
                    self.max_iterations
                );
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
                tools: effective_tools.clone(),
                stream: false,
            };

            // Log request info
            tracing::info!("=== OLLAMA REQUEST (Direct HTTP) ===");
            tracing::info!("Model: {}", self.model);
            tracing::info!("Messages count: {}", messages.len());
            tracing::info!("Tools count: {}", effective_tools.len());

            // Verbose feedback before Ollama call
            if self.verbose {
                eprint!("[       ...] Waiting for {} ", self.model);
                if iterations == 1 {
                    eprintln!("({}msg, {}tools)", messages.len(), effective_tools.len());
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

            let raw_body = response
                .text()
                .await
                .context("Failed to get response text")?;
            let response_body: DirectChatResponse =
                serde_json::from_str(&raw_body).context("Failed to parse Ollama response")?;
            let ollama_elapsed = ollama_start.elapsed();

            let assistant_msg = response_body.message;

            // Log response
            tracing::info!("=== OLLAMA RESPONSE ===");
            tracing::info!("Content length: {}", assistant_msg.content.len());
            if !assistant_msg.content.is_empty() {
                tracing::info!(
                    "Content preview: {}",
                    &assistant_msg.content[..assistant_msg.content.len().min(200)]
                );
            }
            tracing::info!("Tool calls count: {}", assistant_msg.tool_calls.len());

            // Verbose timing output
            if self.verbose {
                let tool_count = if assistant_msg.tool_calls.is_empty() {
                    "final".to_string()
                } else {
                    format!("{} tool call(s)", assistant_msg.tool_calls.len())
                };
                eprintln!(
                    "[{:>7}ms] Ollama response ({})",
                    ollama_elapsed.as_millis(),
                    tool_count
                );
            }
            for (i, tc) in assistant_msg.tool_calls.iter().enumerate() {
                tracing::info!(
                    "Tool call {}: {} args={:?}",
                    i,
                    tc.function.name,
                    tc.function.arguments
                );
            }

            // Check if there are tool calls
            // First check the standard tool_calls array, then fallback to parsing content via registry
            let tool_calls = if !assistant_msg.tool_calls.is_empty() {
                assistant_msg.tool_calls.clone()
            } else if let Some((parsed_call, parser_name)) =
                self.parser_registry.parse(&assistant_msg.content)
            {
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

                // Strip <think>...</think> tags if model uses reasoning traces
                let final_content = if self.capabilities.thinking {
                    let stripped = strip_think_tags(&assistant_msg.content);
                    if stripped.len() != assistant_msg.content.len() {
                        tracing::debug!(
                            "Stripped {} chars of <think> content from response",
                            assistant_msg.content.len() - stripped.len()
                        );
                    }
                    stripped
                } else {
                    assistant_msg.content.clone()
                };

                let total_duration = total_start.elapsed();

                // Emit response complete event
                self.event_sender
                    .response_complete(&final_content, iterations, total_duration);

                if self.verbose {
                    eprintln!(
                        "[{:>7}ms] Total ({} iteration{})",
                        total_duration.as_millis(),
                        iterations,
                        if iterations == 1 { "" } else { "s" }
                    );
                }

                // Add to history (use stripped content so history is clean)
                self.history.push(DirectMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                    tool_calls: None,
                });
                self.history.push(DirectMessage {
                    role: "assistant".to_string(),
                    content: final_content.clone(),
                    tool_calls: None,
                });

                // Prune history to stay within limits
                self.prune_history();

                return Ok(final_content);
            }

            // Process tool calls
            tracing::info!("Agent making {} tool call(s)", tool_calls.len());

            // Add assistant message with tool calls to messages
            messages.push(DirectMessage {
                role: "assistant".to_string(),
                content: assistant_msg.content.clone(),
                tool_calls: Some(tool_calls.clone()),
            });

            // Execute each tool call and add results
            for tool_call in &tool_calls {
                // Emit tool start event
                self.event_sender
                    .tool_start(&tool_call.function.name, &tool_call.function.arguments);

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
                let (result, is_error) = match tokio::time::timeout(
                    self.tool_timeout,
                    tools::execute_tool_call(&mut self.mcp_pool, tool_call),
                )
                .await
                {
                    Ok(Ok(r)) => (r, false),
                    Ok(Err(e)) => (
                        format!("Error calling tool {}: {}", tool_call.function.name, e),
                        true,
                    ),
                    Err(_) => (
                        format!(
                            "Tool {} timed out after {:?}",
                            tool_call.function.name, self.tool_timeout
                        ),
                        true,
                    ),
                };
                let tool_elapsed = tool_start.elapsed();
                let duration_ms = tool_elapsed.as_millis() as u64;

                // Record MCP metrics and classify error
                let server_name = self
                    .mcp_pool
                    .server_for_tool(&tool_call.function.name)
                    .unwrap_or_else(|| "unknown".to_string());
                let error_type = if is_error {
                    let error = metrics::ToolCallError::classify(&result);
                    self.mcp_metrics.record_error(
                        &server_name,
                        &tool_call.function.name,
                        duration_ms,
                        &error,
                    );
                    Some(error.label().to_string())
                } else {
                    self.mcp_metrics.record_success(
                        &server_name,
                        &tool_call.function.name,
                        duration_ms,
                    );
                    None
                };

                // Emit tool complete event
                self.event_sender.tool_complete(
                    &tool_call.function.name,
                    &result,
                    tool_elapsed,
                    is_error,
                    error_type,
                );

                if self.verbose {
                    eprintln!(
                        "[{:>7}ms] → {}",
                        tool_elapsed.as_millis(),
                        tool_call.function.name
                    );
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
            eprintln!(
                "[{:>7}ms] Total ({} iteration{})",
                total_duration.as_millis(),
                iterations,
                if iterations == 1 { "" } else { "s" }
            );
        }

        // Emit error event for max iterations
        let error_msg = format!(
            "Agent reached maximum iterations ({}) without completing",
            self.max_iterations
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

    // ============== Stability Configuration Tests ==============

    #[test]
    fn test_default_stability_config() {
        // Default values should match the constants
        assert_eq!(DEFAULT_MAX_ITERATIONS, 10);
        assert_eq!(DEFAULT_LLM_TIMEOUT_SECS, 300);
        assert_eq!(DEFAULT_TOOL_TIMEOUT_SECS, 60);
        assert_eq!(DEFAULT_MAX_HISTORY_MESSAGES, 100);
    }

    #[test]
    fn test_with_config_custom_values() {
        let pool = McpClientPool::empty();
        let agent = Agent::with_config(
            "http://localhost:11434",
            "test-model",
            pool,
            5,   // max_iterations
            120, // llm_timeout_secs
            30,  // tool_timeout_secs
            50,  // max_history_messages
        );

        assert_eq!(agent.max_iterations(), 5);
        assert_eq!(agent.llm_timeout(), Duration::from_secs(120));
        assert_eq!(agent.tool_timeout(), Duration::from_secs(30));
        assert_eq!(agent.max_history_messages(), 50);
    }

    #[test]
    fn test_prune_history_under_limit() {
        let pool = McpClientPool::empty();
        let mut agent = Agent::with_config(
            "http://localhost:11434",
            "test",
            pool,
            10,
            300,
            60,
            100, // max 100 messages
        );

        // Add 50 messages (under limit)
        for i in 0..50 {
            agent.history.push(DirectMessage {
                role: "user".to_string(),
                content: format!("message {}", i),
                tool_calls: None,
            });
        }

        agent.prune_history();

        // Should not have pruned anything
        assert_eq!(agent.history.len(), 50);
    }

    #[test]
    fn test_prune_history_over_limit() {
        let pool = McpClientPool::empty();
        let mut agent = Agent::with_config(
            "http://localhost:11434",
            "test",
            pool,
            10,
            300,
            60,
            10, // max 10 messages
        );

        // Add 15 messages (over limit)
        for i in 0..15 {
            agent.history.push(DirectMessage {
                role: "user".to_string(),
                content: format!("message {}", i),
                tool_calls: None,
            });
        }

        agent.prune_history();

        // Should have pruned to 10 messages
        assert_eq!(agent.history.len(), 10);
        // Should keep the most recent (messages 5-14)
        assert_eq!(agent.history[0].content, "message 5");
        assert_eq!(agent.history[9].content, "message 14");
    }

    #[test]
    fn test_prune_history_exact_limit() {
        let pool = McpClientPool::empty();
        let mut agent = Agent::with_config(
            "http://localhost:11434",
            "test",
            pool,
            10,
            300,
            60,
            10, // max 10 messages
        );

        // Add exactly 10 messages
        for i in 0..10 {
            agent.history.push(DirectMessage {
                role: "user".to_string(),
                content: format!("message {}", i),
                tool_calls: None,
            });
        }

        agent.prune_history();

        // Should not have pruned anything
        assert_eq!(agent.history.len(), 10);
    }

    // ============== from_agent_config Tests ==============

    #[test]
    fn test_from_agent_config_applies_custom_values() {
        use crate::config::AgentSectionConfig;

        let config = AgentSectionConfig {
            system_prompt: None,
            max_iterations: 3,
            llm_timeout_secs: 120,
            tool_timeout_secs: 30,
            max_history_messages: 50,
            mcp_connect_timeout_secs: 5,
            mcp_startup_timeout_secs: 30,
        };

        let pool = McpClientPool::empty();
        let agent = Agent::from_agent_config("http://localhost:11434", "test-model", pool, &config);

        assert_eq!(agent.max_iterations(), 3);
        assert_eq!(agent.llm_timeout(), Duration::from_secs(120));
        assert_eq!(agent.tool_timeout(), Duration::from_secs(30));
        assert_eq!(agent.max_history_messages(), 50);
    }

    #[test]
    fn test_from_agent_config_with_defaults() {
        use crate::config::AgentSectionConfig;

        let config = AgentSectionConfig::default();
        let pool = McpClientPool::empty();
        let agent = Agent::from_agent_config("http://localhost:11434", "test-model", pool, &config);

        // Should match the DEFAULT_* constants
        assert_eq!(agent.max_iterations(), DEFAULT_MAX_ITERATIONS);
        assert_eq!(
            agent.llm_timeout(),
            Duration::from_secs(DEFAULT_LLM_TIMEOUT_SECS)
        );
        assert_eq!(
            agent.tool_timeout(),
            Duration::from_secs(DEFAULT_TOOL_TIMEOUT_SECS)
        );
        assert_eq!(agent.max_history_messages(), DEFAULT_MAX_HISTORY_MESSAGES);
    }

    #[test]
    fn test_from_agent_config_matches_with_config() {
        use crate::config::AgentSectionConfig;

        // Verify from_agent_config produces the same agent as with_config
        let config = AgentSectionConfig {
            system_prompt: None,
            max_iterations: 7,
            llm_timeout_secs: 200,
            tool_timeout_secs: 45,
            max_history_messages: 75,
            mcp_connect_timeout_secs: 5,
            mcp_startup_timeout_secs: 30,
        };

        let pool1 = McpClientPool::empty();
        let agent1 =
            Agent::from_agent_config("http://localhost:11434", "test-model", pool1, &config);

        let pool2 = McpClientPool::empty();
        let agent2 = Agent::with_config(
            "http://localhost:11434",
            "test-model",
            pool2,
            7,
            200,
            45,
            75,
        );

        assert_eq!(agent1.max_iterations(), agent2.max_iterations());
        assert_eq!(agent1.llm_timeout(), agent2.llm_timeout());
        assert_eq!(agent1.tool_timeout(), agent2.tool_timeout());
        assert_eq!(agent1.max_history_messages(), agent2.max_history_messages());
    }
}
