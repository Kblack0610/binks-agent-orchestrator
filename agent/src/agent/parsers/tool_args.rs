//! Alternative tool call format parser (tool/args)
//!
//! Handles format: {"tool": "...", "args": {...}}
//! Some models use this alternative naming convention.

use serde::Deserialize;

use super::{ToolCall, ToolCallFunction, ToolCallParser};

/// Parser for tool/args format
///
/// Format: `{"tool": "tool_name", "args": {"key": "value"}}`
pub struct ToolArgsParser;

#[derive(Deserialize)]
struct ToolArgsFormat {
    tool: String,
    #[serde(default)]
    args: serde_json::Value,
}

impl ToolCallParser for ToolArgsParser {
    fn parse(&self, content: &str) -> Option<ToolCall> {
        let parsed: ToolArgsFormat = serde_json::from_str(content).ok()?;

        // Normalize null args to empty object
        let arguments = if parsed.args.is_null() {
            serde_json::json!({})
        } else {
            parsed.args
        };

        Some(ToolCall {
            id: None,
            function: ToolCallFunction {
                index: None,
                name: parsed.tool,
                arguments,
            },
        })
    }

    fn name(&self) -> &'static str {
        "ToolArgsParser"
    }

    fn priority(&self) -> u32 {
        50 // Lower priority - alternative format
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid() {
        let parser = ToolArgsParser;
        let content = r#"{"tool": "get_weather", "args": {"city": "NYC"}}"#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        assert_eq!(tool_call.function.name, "get_weather");
        assert_eq!(tool_call.function.arguments["city"], "NYC");
    }

    #[test]
    fn test_parse_missing_args() {
        let parser = ToolArgsParser;
        let content = r#"{"tool": "get_cpu_usage"}"#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        assert_eq!(tool_call.function.name, "get_cpu_usage");
        // Should default to empty object
        assert!(tool_call.function.arguments.is_object());
    }

    #[test]
    fn test_parse_null_args() {
        let parser = ToolArgsParser;
        let content = r#"{"tool": "test", "args": null}"#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        // Null should be normalized to empty object
        assert!(tool_call.function.arguments.is_object());
        assert!(!tool_call.function.arguments.is_null());
    }

    #[test]
    fn test_parse_invalid() {
        let parser = ToolArgsParser;

        // Missing tool
        assert!(parser.parse(r#"{"args": {}}"#).is_none());

        // Wrong format (standard format)
        assert!(parser.parse(r#"{"name": "test", "arguments": {}}"#).is_none());
    }
}
