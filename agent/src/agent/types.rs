//! Direct API types for Ollama
//!
//! These types are used for direct HTTP communication with Ollama,
//! bypassing ollama-rs for better tool calling support.

use serde::{Deserialize, Serialize};

use super::parsers::ToolCall;

/// Direct chat request for Ollama API
#[derive(Debug, Serialize)]
pub struct DirectChatRequest {
    pub model: String,
    pub messages: Vec<DirectMessage>,
    pub tools: Vec<DirectTool>,
    pub stream: bool,
}

/// A message in the conversation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl DirectMessage {
    /// Create a new message
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_calls: None,
        }
    }
}

/// A tool definition for Ollama
#[derive(Debug, Serialize, Clone)]
pub struct DirectTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: DirectToolFunction,
}

/// Function definition within a tool
#[derive(Debug, Serialize, Clone)]
pub struct DirectToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Response from Ollama chat endpoint
#[derive(Debug, Deserialize)]
pub struct DirectChatResponse {
    pub message: DirectResponseMessage,
}

/// Message in the response
#[derive(Debug, Deserialize)]
pub struct DirectResponseMessage {
    #[allow(dead_code)]
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}
