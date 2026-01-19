//! Standard tool call format parser
//!
//! Handles the most common format: {"name": "...", "arguments": {...}}
//! This is the OpenAI-compatible format used by most models.

use serde::Deserialize;

use super::{ToolCall, ToolCallFunction, ToolCallParser};

/// Parser for standard OpenAI-compatible format
///
/// Format: `{"name": "tool_name", "arguments": {"key": "value"}}`
pub struct StandardParser;

#[derive(Deserialize)]
struct StandardFormat {
    name: String,
    arguments: serde_json::Value,
}

impl ToolCallParser for StandardParser {
    fn parse(&self, content: &str) -> Option<ToolCall> {
        let parsed: StandardFormat = serde_json::from_str(content).ok()?;

        Some(ToolCall {
            id: None,
            function: ToolCallFunction {
                index: None,
                name: parsed.name,
                arguments: parsed.arguments,
            },
        })
    }

    fn name(&self) -> &'static str {
        "StandardParser"
    }

    fn priority(&self) -> u32 {
        100 // Highest priority - standard format
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid() {
        let parser = StandardParser;
        let content = r#"{"name": "get_weather", "arguments": {"city": "NYC"}}"#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        assert_eq!(tool_call.function.name, "get_weather");
        assert_eq!(tool_call.function.arguments["city"], "NYC");
    }

    #[test]
    fn test_parse_empty_arguments() {
        let parser = StandardParser;
        let content = r#"{"name": "get_cpu_usage", "arguments": {}}"#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        assert_eq!(tool_call.function.name, "get_cpu_usage");
        assert!(tool_call.function.arguments.is_object());
    }

    #[test]
    fn test_parse_invalid() {
        let parser = StandardParser;

        // Missing name
        assert!(parser.parse(r#"{"arguments": {}}"#).is_none());

        // Wrong format
        assert!(parser.parse(r#"{"tool": "test"}"#).is_none());

        // Invalid JSON
        assert!(parser.parse("not json").is_none());
    }
}
