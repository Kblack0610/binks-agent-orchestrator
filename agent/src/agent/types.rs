//! Direct API types for the LiteLLM gateway.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::parsers::ToolCall;

/// Direct chat request for OpenAI-compatible chat completions.
#[derive(Debug, Serialize)]
pub struct DirectChatRequest {
    pub model: String,
    pub messages: Vec<DirectMessage>,
    /// Tools array - omitted from JSON when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<DirectTool>,
    pub stream: bool,
}

/// A message in the conversation.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectMessage {
    pub role: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_message_content"
    )]
    pub content: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_tool_calls"
    )]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl DirectMessage {
    /// Create a new message.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

/// A tool definition for OpenAI-compatible chat completions.
#[derive(Debug, Serialize, Clone)]
pub struct DirectTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: DirectToolFunction,
}

/// Function definition within a tool.
#[derive(Debug, Serialize, Clone)]
pub struct DirectToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Response from the gateway chat endpoint.
#[derive(Debug, Deserialize)]
pub struct DirectChatResponse {
    pub choices: Vec<DirectChoice>,
}

/// Choice wrapper in the gateway response.
#[derive(Debug, Deserialize)]
pub struct DirectChoice {
    pub message: DirectResponseMessage,
}

/// Message in the response.
#[derive(Debug, Deserialize)]
pub struct DirectResponseMessage {
    #[allow(dead_code)]
    pub role: String,
    #[serde(default, deserialize_with = "deserialize_message_content")]
    pub content: String,
    #[serde(default, deserialize_with = "deserialize_tool_calls")]
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Serialize)]
struct RequestToolCall<'a> {
    id: String,
    #[serde(rename = "type")]
    tool_type: &'static str,
    function: RequestToolCallFunction<'a>,
}

#[derive(Debug, Serialize)]
struct RequestToolCallFunction<'a> {
    name: &'a str,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct ResponseToolCall {
    id: Option<String>,
    function: ResponseToolCallFunction,
}

#[derive(Debug, Deserialize)]
struct ResponseToolCallFunction {
    name: String,
    arguments: serde_json::Value,
}

fn deserialize_message_content<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    let content = match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(text) => text,
        serde_json::Value::Array(items) => items
            .into_iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .collect::<Vec<_>>()
            .join(""),
        other => other.to_string(),
    };

    Ok(content)
}

fn serialize_tool_calls<S>(
    tool_calls: &Option<Vec<ToolCall>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match tool_calls {
        Some(tool_calls) => {
            let payload: Vec<RequestToolCall<'_>> = tool_calls
                .iter()
                .enumerate()
                .map(|(index, tool_call)| RequestToolCall {
                    id: tool_call
                        .id
                        .clone()
                        .unwrap_or_else(|| format!("call_{}", index)),
                    tool_type: "function",
                    function: RequestToolCallFunction {
                        name: &tool_call.function.name,
                        arguments: tool_call.function.arguments.to_string(),
                    },
                })
                .collect();

            payload.serialize(serializer)
        }
        None => serializer.serialize_none(),
    }
}

fn deserialize_tool_calls<'de, D>(deserializer: D) -> Result<Vec<ToolCall>, D::Error>
where
    D: Deserializer<'de>,
{
    let response_tool_calls = Vec::<ResponseToolCall>::deserialize(deserializer)?;

    response_tool_calls
        .into_iter()
        .map(|tool_call| {
            let arguments = match tool_call.function.arguments {
                serde_json::Value::String(raw) => {
                    serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::Value::String(raw))
                }
                other => other,
            };

            Ok(ToolCall {
                id: tool_call.id,
                function: super::parsers::ToolCallFunction {
                    index: None,
                    name: tool_call.function.name,
                    arguments,
                },
            })
        })
        .collect()
}
