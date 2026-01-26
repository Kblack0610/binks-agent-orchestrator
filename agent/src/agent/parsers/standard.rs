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

    // ============== Edge Case Tests ==============

    #[test]
    fn test_parse_nested_json_arguments() {
        let parser = StandardParser;
        let content = r#"{"name": "create_resource", "arguments": {
            "config": {
                "nested": {
                    "deep": {"value": 42}
                },
                "array": [1, 2, 3]
            }
        }}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.name, "create_resource");
        assert_eq!(
            result.function.arguments["config"]["nested"]["deep"]["value"],
            42
        );
        assert_eq!(result.function.arguments["config"]["array"][0], 1);
    }

    #[test]
    fn test_parse_unicode_in_tool_name() {
        let parser = StandardParser;
        let content = r#"{"name": "获取天气", "arguments": {"城市": "北京"}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.name, "获取天气");
        assert_eq!(result.function.arguments["城市"], "北京");
    }

    #[test]
    fn test_parse_unicode_emoji_in_arguments() {
        let parser = StandardParser;
        let content = r#"{"name": "send_message", "arguments": {"text": "Hello 👋 World 🌍"}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.arguments["text"], "Hello 👋 World 🌍");
    }

    #[test]
    fn test_parse_special_characters_in_arguments() {
        let parser = StandardParser;
        // Test with newlines and tabs in the value
        let content =
            r#"{"name": "run_query", "arguments": {"sql": "SELECT *\nFROM users\tWHERE id = 1"}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.name, "run_query");
        assert!(result.function.arguments["sql"]
            .as_str()
            .unwrap()
            .contains("SELECT"));
        assert!(result.function.arguments["sql"]
            .as_str()
            .unwrap()
            .contains('\n'));
    }

    #[test]
    fn test_parse_array_arguments() {
        let parser = StandardParser;
        let content = r#"{"name": "batch_process", "arguments": {"items": ["a", "b", "c"], "ids": [1, 2, 3]}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.arguments["items"][0], "a");
        assert_eq!(result.function.arguments["ids"][2], 3);
    }

    #[test]
    fn test_parse_mixed_type_arguments() {
        let parser = StandardParser;
        let content = r#"{"name": "config", "arguments": {
            "string": "hello",
            "number": 42,
            "float": 3.15,
            "bool": true,
            "null_val": null,
            "array": [1, "two", false]
        }}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.arguments["string"], "hello");
        assert_eq!(result.function.arguments["number"], 42);
        assert_eq!(result.function.arguments["float"], 3.15);
        assert_eq!(result.function.arguments["bool"], true);
        assert!(result.function.arguments["null_val"].is_null());
    }

    #[test]
    fn test_parse_whitespace_variations() {
        let parser = StandardParser;

        // Minified
        let minified = r#"{"name":"test","arguments":{"key":"value"}}"#;
        assert!(parser.parse(minified).is_some());

        // Extra whitespace
        let spaced = r#"{  "name"  :  "test"  ,  "arguments"  :  {  }  }"#;
        assert!(parser.parse(spaced).is_some());

        // With newlines
        let multiline = "{\n  \"name\": \"test\",\n  \"arguments\": {}\n}";
        assert!(parser.parse(multiline).is_some());
    }

    #[test]
    fn test_parse_long_tool_name() {
        let parser = StandardParser;
        let long_name = "a".repeat(1000);
        let content = format!(r#"{{"name": "{}", "arguments": {{}}}}"#, long_name);

        let result = parser.parse(&content).unwrap();
        assert_eq!(result.function.name.len(), 1000);
    }

    #[test]
    fn test_parse_long_argument_value() {
        let parser = StandardParser;
        let long_value = "x".repeat(10000);
        let content = format!(
            r#"{{"name": "test", "arguments": {{"data": "{}"}}}}"#,
            long_value
        );

        let result = parser.parse(&content).unwrap();
        assert_eq!(
            result.function.arguments["data"].as_str().unwrap().len(),
            10000
        );
    }

    #[test]
    fn test_parse_empty_string_name_fails() {
        let parser = StandardParser;
        // Empty name should still parse (validation is separate concern)
        let content = r#"{"name": "", "arguments": {}}"#;
        let result = parser.parse(content);
        // Parser succeeds, semantic validation would reject empty name
        assert!(result.is_some());
        assert_eq!(result.unwrap().function.name, "");
    }

    #[test]
    fn test_parse_null_arguments_value() {
        let parser = StandardParser;
        let content = r#"{"name": "test", "arguments": null}"#;

        let result = parser.parse(content).unwrap();
        assert!(result.function.arguments.is_null());
    }

    #[test]
    fn test_parse_arguments_as_array_preserved() {
        let parser = StandardParser;
        // Some models might send array as arguments (unusual but valid JSON)
        let content = r#"{"name": "test", "arguments": ["item1", "item2"]}"#;

        let result = parser.parse(content).unwrap();
        assert!(result.function.arguments.is_array());
    }

    #[test]
    fn test_parse_extra_fields_ignored() {
        let parser = StandardParser;
        // Extra fields should be ignored gracefully
        let content = r#"{"name": "test", "arguments": {}, "extra": "ignored", "id": "123"}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.name, "test");
    }
}
