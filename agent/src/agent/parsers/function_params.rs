//! Alternative tool call format parser (function/parameters)
//!
//! Handles format: {"function": "...", "parameters": {...}}
//! Some models use this alternative naming convention.

use serde::Deserialize;

use super::{ToolCall, ToolCallFunction, ToolCallParser};

/// Parser for function/parameters format
///
/// Format: `{"function": "tool_name", "parameters": {"key": "value"}}`
pub struct FunctionParamsParser;

#[derive(Deserialize)]
struct FunctionParamsFormat {
    function: String,
    #[serde(default)]
    parameters: serde_json::Value,
}

impl ToolCallParser for FunctionParamsParser {
    fn parse(&self, content: &str) -> Option<ToolCall> {
        let parsed: FunctionParamsFormat = serde_json::from_str(content).ok()?;

        // Normalize null parameters to empty object
        let arguments = if parsed.parameters.is_null() {
            serde_json::json!({})
        } else {
            parsed.parameters
        };

        Some(ToolCall {
            id: None,
            function: ToolCallFunction {
                index: None,
                name: parsed.function,
                arguments,
            },
        })
    }

    fn name(&self) -> &'static str {
        "FunctionParamsParser"
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
        let parser = FunctionParamsParser;
        let content = r#"{"function": "get_weather", "parameters": {"city": "NYC"}}"#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        assert_eq!(tool_call.function.name, "get_weather");
        assert_eq!(tool_call.function.arguments["city"], "NYC");
    }

    #[test]
    fn test_parse_missing_parameters() {
        let parser = FunctionParamsParser;
        let content = r#"{"function": "get_cpu_usage"}"#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        assert_eq!(tool_call.function.name, "get_cpu_usage");
        // Should default to empty object
        assert!(tool_call.function.arguments.is_object());
    }

    #[test]
    fn test_parse_null_parameters() {
        let parser = FunctionParamsParser;
        let content = r#"{"function": "test", "parameters": null}"#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        // Null should be normalized to empty object
        assert!(tool_call.function.arguments.is_object());
        assert!(!tool_call.function.arguments.is_null());
    }

    #[test]
    fn test_parse_invalid() {
        let parser = FunctionParamsParser;

        // Missing function
        assert!(parser.parse(r#"{"parameters": {}}"#).is_none());

        // Wrong format (standard format)
        assert!(parser
            .parse(r#"{"name": "test", "arguments": {}}"#)
            .is_none());
    }

    // ============== Edge Case Tests ==============

    #[test]
    fn test_parse_nested_parameters() {
        let parser = FunctionParamsParser;
        let content = r#"{"function": "deploy", "parameters": {"config": {"replicas": 3, "env": {"DEBUG": "true"}}}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.arguments["config"]["replicas"], 3);
        assert_eq!(result.function.arguments["config"]["env"]["DEBUG"], "true");
    }

    #[test]
    fn test_parse_array_parameters() {
        let parser = FunctionParamsParser;
        let content = r#"{"function": "select", "parameters": {"ids": [1, 2, 3], "tags": ["prod", "active"]}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.arguments["ids"][2], 3);
        assert_eq!(result.function.arguments["tags"][0], "prod");
    }

    #[test]
    fn test_parse_unicode_content() {
        let parser = FunctionParamsParser;
        let content = r#"{"function": "translate", "parameters": {"text": "Привет мир 🌍", "lang": "日本語"}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.arguments["text"], "Привет мир 🌍");
        assert_eq!(result.function.arguments["lang"], "日本語");
    }

    #[test]
    fn test_parse_empty_function_name() {
        let parser = FunctionParamsParser;
        let content = r#"{"function": "", "parameters": {}}"#;

        // Parser succeeds (validation is separate)
        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.name, "");
    }

    #[test]
    fn test_parse_all_json_types() {
        let parser = FunctionParamsParser;
        let content = r#"{"function": "types", "parameters": {
            "string": "text",
            "integer": 100,
            "float": 2.718,
            "boolean": true,
            "null_value": null,
            "array": [1, "two", false],
            "object": {"key": "value"}
        }}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.arguments["string"], "text");
        assert_eq!(result.function.arguments["integer"], 100);
        assert!(result.function.arguments["boolean"] == true);
        assert!(result.function.arguments["null_value"].is_null());
        assert!(result.function.arguments["array"].is_array());
        assert!(result.function.arguments["object"].is_object());
    }

    #[test]
    fn test_parse_whitespace_formats() {
        let parser = FunctionParamsParser;

        // Compact
        assert!(parser
            .parse(r#"{"function":"f","parameters":{}}"#)
            .is_some());

        // Spaced
        assert!(parser
            .parse(r#"{  "function" : "f" , "parameters" : {} }"#)
            .is_some());

        // With newlines and tabs
        let content = "{\n\t\"function\": \"f\",\n\t\"parameters\": {}\n}";
        assert!(parser.parse(content).is_some());
    }

    #[test]
    fn test_parse_long_strings() {
        let parser = FunctionParamsParser;
        let long_name = "func_".to_string() + &"x".repeat(200);
        let long_value = "v".repeat(5000);
        let content = format!(
            r#"{{"function": "{}", "parameters": {{"data": "{}"}}}}"#,
            long_name, long_value
        );

        let result = parser.parse(&content).unwrap();
        assert!(result.function.name.len() > 200);
        assert_eq!(
            result.function.arguments["data"].as_str().unwrap().len(),
            5000
        );
    }

    #[test]
    fn test_parse_extra_fields_ignored() {
        let parser = FunctionParamsParser;
        let content = r#"{"function": "test", "parameters": {}, "extra": true, "id": "abc"}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.name, "test");
    }

    #[test]
    fn test_parse_mcp_style_function_name() {
        let parser = FunctionParamsParser;
        let content =
            r#"{"function": "mcp__kubernetes__pods_list", "parameters": {"namespace": "default"}}"#;

        let result = parser.parse(content).unwrap();
        assert_eq!(result.function.name, "mcp__kubernetes__pods_list");
        assert_eq!(result.function.arguments["namespace"], "default");
    }

    #[test]
    fn test_parse_parameters_as_primitive_preserved() {
        let parser = FunctionParamsParser;
        // Unusual but parser handles it
        let content = r#"{"function": "test", "parameters": 42}"#;

        let result = parser.parse(content).unwrap();
        // Primitive value preserved (not normalized to object in this case)
        assert_eq!(result.function.arguments, 42);
    }
}
