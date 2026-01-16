//! Agent module - LLM with tool-calling capabilities
//!
//! This implements the "tool-using agent loop" where:
//! 1. User sends a message
//! 2. LLM receives the message along with available tools
//! 3. LLM decides whether to call tools or respond directly
//! 4. If tools are called, results are fed back to LLM
//! 5. Loop continues until LLM responds without tool calls

use anyhow::{Context, Result};
use ollama_rs::{
    generation::{
        chat::{request::ChatMessageRequest, ChatMessage},
        tools::{ToolCall, ToolFunctionInfo, ToolInfo, ToolType},
    },
    Ollama,
};
use schemars::Schema;

use crate::mcp::{McpClientPool, McpTool};

/// Maximum number of tool-calling iterations to prevent infinite loops
const MAX_ITERATIONS: usize = 10;

/// An agent that can use tools via MCP
pub struct Agent {
    ollama: Ollama,
    model: String,
    mcp_pool: McpClientPool,
    system_prompt: Option<String>,
    history: Vec<ChatMessage>,
}

impl Agent {
    /// Create a new agent
    pub fn new(ollama_url: &str, model: &str, mcp_pool: McpClientPool) -> Self {
        let url = url::Url::parse(ollama_url).unwrap_or_else(|_| {
            url::Url::parse("http://localhost:11434").unwrap()
        });

        let host = url.host_str().unwrap_or("localhost").to_string();
        let port = url.port().unwrap_or(11434);

        Self {
            ollama: Ollama::new(format!("http://{}", host), port),
            model: model.to_string(),
            mcp_pool,
            system_prompt: None,
            history: Vec::new(),
        }
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = Some(prompt.to_string());
        self
    }

    /// Convert MCP tools to Ollama ToolInfo format
    fn mcp_tools_to_ollama(tools: &[McpTool]) -> Vec<ToolInfo> {
        tools
            .iter()
            .map(|tool| {
                // Convert the JSON schema to schemars Schema
                let parameters = tool
                    .input_schema
                    .clone()
                    .map(|schema| {
                        // Try to deserialize the JSON value into a Schema
                        serde_json::from_value::<Schema>(schema).unwrap_or_else(|_| {
                            // Fallback to empty object schema
                            serde_json::from_value(serde_json::json!({
                                "type": "object",
                                "properties": {}
                            }))
                            .unwrap()
                        })
                    })
                    .unwrap_or_else(|| {
                        serde_json::from_value(serde_json::json!({
                            "type": "object",
                            "properties": {}
                        }))
                        .unwrap()
                    });

                ToolInfo {
                    tool_type: ToolType::Function,
                    function: ToolFunctionInfo {
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
    pub async fn chat(&mut self, user_message: &str) -> Result<String> {
        // Get available tools from MCP
        let mcp_tools = self.mcp_pool.list_all_tools().await?;
        let tools = Self::mcp_tools_to_ollama(&mcp_tools);

        tracing::info!("Agent has {} tools available", tools.len());

        // Build initial messages
        let mut messages = Vec::new();

        // Add system prompt if set
        if let Some(ref system) = self.system_prompt {
            messages.push(ChatMessage::system(system.clone()));
        }

        // Add history
        messages.extend(self.history.clone());

        // Add new user message
        messages.push(ChatMessage::user(user_message.to_string()));

        // Tool-calling loop
        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                tracing::warn!("Agent reached max iterations ({}), stopping", MAX_ITERATIONS);
                break;
            }

            tracing::debug!("Agent iteration {}", iterations);

            // Create chat request with tools
            let request = ChatMessageRequest::new(self.model.clone(), messages.clone())
                .tools(tools.clone());

            // Send to LLM
            let response = self
                .ollama
                .send_chat_messages(request)
                .await
                .map_err(|e| {
                    tracing::error!("Ollama error: {:?}", e);
                    e
                })
                .context("Failed to send chat request")?;

            let assistant_msg = response.message;

            // Check if there are tool calls
            if assistant_msg.tool_calls.is_empty() {
                // No tool calls - we're done
                tracing::info!("Agent responding without tool calls");

                // Add assistant response to history
                self.history.push(ChatMessage::user(user_message.to_string()));
                self.history.push(ChatMessage::assistant(assistant_msg.content.clone()));

                return Ok(assistant_msg.content);
            }

            // Process tool calls
            tracing::info!(
                "Agent making {} tool call(s)",
                assistant_msg.tool_calls.len()
            );

            // Add assistant message with tool calls to messages
            messages.push(assistant_msg.clone());

            // Execute each tool call and add results
            for tool_call in &assistant_msg.tool_calls {
                let result = match self.execute_tool_call(tool_call).await {
                    Ok(r) => r,
                    Err(e) => {
                        // Return error as tool result so LLM can handle it
                        format!("Error calling tool {}: {}", tool_call.function.name, e)
                    }
                };

                // Create tool response message
                let mut tool_msg = ChatMessage::tool(result);
                // Set tool_call info if needed (some models require this)
                tool_msg.tool_calls = vec![tool_call.clone()];

                messages.push(tool_msg);
            }
        }

        // If we hit max iterations, return last assistant message
        Ok("Agent reached maximum iterations without completing.".to_string())
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

        let ollama_tools = Agent::mcp_tools_to_ollama(&[mcp_tool]);

        assert_eq!(ollama_tools.len(), 1);
        assert_eq!(ollama_tools[0].function.name, "get_weather");
        assert_eq!(
            ollama_tools[0].function.description,
            "Get weather for a city"
        );
    }
}
