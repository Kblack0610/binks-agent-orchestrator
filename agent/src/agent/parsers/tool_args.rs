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

    // ============== Edge Case Tests ==============

    #[test]
    fn test_parse_nested_args() {
        let parser = ToolArgsParser;
        let content = r#"{"tool": "config", "args": {"settings": {"nested": {"deep": true}}}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.arguments["settings"]["nested"]["deep"], true);
    }

    #[test]
    fn test_parse_array_in_args() {
        let parser = ToolArgsParser;
        let content = r#"{"tool": "batch", "args": {"items": [1, 2, 3], "names": ["a", "b"]}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.arguments["items"][0], 1);
        assert_eq!(result.function.arguments["names"][1], "b");
    }

    #[test]
    fn test_parse_unicode() {
        let parser = ToolArgsParser;
        let content = r#"{"tool": "message", "args": {"text": "Hello 你好 مرحبا 👋"}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.arguments["text"], "Hello 你好 مرحبا 👋");
    }

    #[test]
    fn test_parse_empty_tool_name() {
        let parser = ToolArgsParser;
        let content = r#"{"tool": "", "args": {}}"#;

        // Parser succeeds (validation is separate)
        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.name, "");
    }

    #[test]
    fn test_parse_mixed_types_in_args() {
        let parser = ToolArgsParser;
        let content = r#"{"tool": "test", "args": {
            "str": "hello",
            "num": 42,
            "float": 3.14159,
            "bool": false,
            "null": null
        }}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.arguments["str"], "hello");
        assert_eq!(result.function.arguments["num"], 42);
        assert!(result.function.arguments["bool"] == false);
        assert!(result.function.arguments["null"].is_null());
    }

    #[test]
    fn test_parse_whitespace_variations() {
        let parser = ToolArgsParser;

        // Minified
        assert!(parser.parse(r#"{"tool":"t","args":{}}"#).is_some());

        // Extra spaces
        assert!(parser.parse(r#"{  "tool"  :  "t"  ,  "args"  :  {}  }"#).is_some());

        // Multiline
        let multiline = "{\n  \"tool\": \"t\",\n  \"args\": {}\n}";
        assert!(parser.parse(multiline).is_some());
    }

    #[test]
    fn test_parse_long_tool_name() {
        let parser = ToolArgsParser;
        let long_name = "x".repeat(500);
        let content = format!(r#"{{"tool": "{}", "args": {{}}}}"#, long_name);

        let result = parser.parse(&content).unwrap();
        assert_eq!(result.function.name.len(), 500);
    }

    #[test]
    fn test_parse_extra_fields_ignored() {
        let parser = ToolArgsParser;
        let content = r#"{"tool": "test", "args": {}, "id": "123", "extra": "data"}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.name, "test");
    }

    #[test]
    fn test_parse_args_as_string_fails() {
        let parser = ToolArgsParser;
        // If args is a string instead of object, it's not valid for our use
        // but serde_json::Value accepts it
        let content = r#"{"tool": "test", "args": "not an object"}"#;

        let result = parser.parse(content).unwrap();
        // Parser succeeds, args preserved as string value
        assert!(result.function.arguments.is_string());
    }

    #[test]
    fn test_parse_special_chars_in_tool_name() {
        let parser = ToolArgsParser;
        let content = r#"{"tool": "mcp__server__tool_name", "args": {}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.name, "mcp__server__tool_name");
    }
}
