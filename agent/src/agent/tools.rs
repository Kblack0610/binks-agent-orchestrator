//! Tool-related utilities for the Agent
//!
//! This module contains helpers for:
//! - Converting MCP tools to Ollama format
//! - Cleaning JSON schemas for Ollama compatibility
//! - Executing tool calls via MCP

use anyhow::{Context, Result};
use rmcp::model::CallToolResult;

use crate::mcp::{McpClientPool, McpTool};
use super::parsers::ToolCall;
use super::types::{DirectTool, DirectToolFunction};

/// Clean up a JSON schema for Ollama compatibility
/// Removes $schema, title, and other fields that confuse Ollama
pub fn clean_schema_for_ollama(schema: &serde_json::Value) -> serde_json::Value {
    match schema {
        serde_json::Value::Object(obj) => {
            let mut cleaned = serde_json::Map::new();
            for (key, value) in obj {
                // Skip fields that Ollama doesn't expect
                if key == "$schema" || key == "title" || key == "additionalProperties" {
                    continue;
                }
                // Recursively clean nested objects (like properties)
                cleaned.insert(key.clone(), clean_schema_for_ollama(value));
            }
            serde_json::Value::Object(cleaned)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(clean_schema_for_ollama).collect())
        }
        other => other.clone(),
    }
}

/// Convert MCP tools to direct API format
pub fn mcp_tools_to_direct(tools: &[McpTool]) -> Vec<DirectTool> {
    tools
        .iter()
        .map(|tool| {
            let parameters = tool
                .input_schema
                .clone()
                .map(|schema| clean_schema_for_ollama(&schema))
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

/// Execute a tool call via MCP and return the result as a string
pub async fn execute_tool_call(
    mcp_pool: &mut McpClientPool,
    tool_call: &ToolCall,
) -> Result<String> {
    let name = &tool_call.function.name;
    let args = &tool_call.function.arguments;

    tracing::info!("Executing tool: {} with args: {:?}", name, args);

    // Call the tool via MCP
    let result: CallToolResult = mcp_pool
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
