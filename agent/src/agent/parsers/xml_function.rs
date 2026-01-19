//! XML function call format parser
//!
//! Handles XML-style tool calls that some models (like qwen) output:
//! `<function=tool_name><parameter=key>value</parameter></function>`
//!
//! This format is commonly seen when models don't properly use JSON tool calling.

use regex::Regex;

use super::{ToolCall, ToolCallFunction, ToolCallParser};

/// Parser for XML-style function calls
///
/// Format: `<function=tool_name><parameter=key>value</parameter></function>`
/// Also handles empty params: `<function=tool_name></function>`
pub struct XmlFunctionParser {
    function_re: Regex,
    param_re: Regex,
}

impl Default for XmlFunctionParser {
    fn default() -> Self {
        Self::new()
    }
}

impl XmlFunctionParser {
    pub fn new() -> Self {
        Self {
            // Match <function=NAME>BODY</function>
            // Using [\s\S]*? for non-greedy match of any content including newlines
            function_re: Regex::new(r"<function=([^>]+)>([\s\S]*?)</function>").unwrap(),
            // Match <parameter=KEY>VALUE</parameter>
            param_re: Regex::new(r"<parameter=([^>]+)>([^<]*)</parameter>").unwrap(),
        }
    }
}

impl ToolCallParser for XmlFunctionParser {
    fn parse(&self, content: &str) -> Option<ToolCall> {
        let caps = self.function_re.captures(content)?;
        let name = caps.get(1)?.as_str().trim().to_string();
        let body = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        // Extract parameters from body
        let mut args = serde_json::Map::new();
        for param_caps in self.param_re.captures_iter(body) {
            if let (Some(key), Some(value)) = (param_caps.get(1), param_caps.get(2)) {
                let key = key.as_str().trim();
                let value = value.as_str().trim();

                // Try to parse value as JSON first, fall back to string
                let json_value = serde_json::from_str(value)
                    .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));

                args.insert(key.to_string(), json_value);
            }
        }

        Some(ToolCall {
            id: None,
            function: ToolCallFunction {
                index: None,
                name,
                arguments: serde_json::Value::Object(args),
            },
        })
    }

    fn name(&self) -> &'static str {
        "XmlFunctionParser"
    }

    fn priority(&self) -> u32 {
        75 // Between StandardParser (100) and ToolArgsParser (50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let parser = XmlFunctionParser::new();
        let content = r#"<function=list_allowed_directories>
</function>"#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        assert_eq!(tool_call.function.name, "list_allowed_directories");
        assert!(tool_call.function.arguments.is_object());
    }

    #[test]
    fn test_parse_with_params() {
        let parser = XmlFunctionParser::new();
        let content = r#"<function=list_dir>
<parameter=path>/home/user</parameter>
<parameter=recursive>true</parameter>
</function>"#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        assert_eq!(tool_call.function.name, "list_dir");
        assert_eq!(tool_call.function.arguments["path"], "/home/user");
        // Note: "true" as string since we parse it as string first
    }

    #[test]
    fn test_parse_with_trailing_content() {
        let parser = XmlFunctionParser::new();
        // Some models add </tool_call> after </function>
        let content = r#"<function=get_weather>
<parameter=city>NYC</parameter>
</function>
</tool_call>"#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        assert_eq!(tool_call.function.name, "get_weather");
        assert_eq!(tool_call.function.arguments["city"], "NYC");
    }

    #[test]
    fn test_parse_no_match() {
        let parser = XmlFunctionParser::new();

        // Regular text
        assert!(parser.parse("hello world").is_none());

        // JSON format (not XML)
        assert!(parser.parse(r#"{"name": "test"}"#).is_none());

        // Incomplete XML
        assert!(parser.parse("<function=test>").is_none());
    }

    #[test]
    fn test_parse_embedded_in_text() {
        let parser = XmlFunctionParser::new();
        let content = r#"I'll help you with that.

<function=read_file>
<parameter=path>/etc/hosts</parameter>
</function>

Let me know if you need anything else."#;

        let result = parser.parse(content);
        assert!(result.is_some());

        let tool_call = result.unwrap();
        assert_eq!(tool_call.function.name, "read_file");
        assert_eq!(tool_call.function.arguments["path"], "/etc/hosts");
    }
}
