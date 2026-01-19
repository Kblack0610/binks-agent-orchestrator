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
        assert!(parser.parse(r#"{"name": "test", "arguments": {}}"#).is_none());
    }
}
