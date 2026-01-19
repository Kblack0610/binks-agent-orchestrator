//! Modular tool call parser registry
//!
//! This module provides a trait-based system for parsing tool calls from LLM responses.
//! Different models output tool calls in different formats - some use the standard
//! `tool_calls` array, others embed JSON in the content field.
//!
//! The registry tries parsers in priority order, making it easy to add new formats.

use serde::{Deserialize, Serialize};

mod standard;
mod tool_args;
mod function_params;
mod xml_function;

pub use standard::StandardParser;
pub use tool_args::ToolArgsParser;
pub use function_params::FunctionParamsParser;
pub use xml_function::XmlFunctionParser;

// ============================================================================
// Shared Types (used by agent and parsers)
// ============================================================================

/// A tool call parsed from LLM response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub function: ToolCallFunction,
}

/// The function details within a tool call
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallFunction {
    #[serde(default)]
    pub index: Option<i32>,
    pub name: String,
    pub arguments: serde_json::Value,
}

// ============================================================================
// Parser Trait
// ============================================================================

/// Trait for parsing tool calls from content
///
/// Implement this trait to add support for new tool call formats.
/// Each parser should handle one specific format.
pub trait ToolCallParser: Send + Sync {
    /// Attempt to parse content as a tool call
    ///
    /// Returns `Some(ToolCall)` if the content matches this parser's format,
    /// `None` otherwise.
    fn parse(&self, content: &str) -> Option<ToolCall>;

    /// Parser name for logging/debugging
    fn name(&self) -> &'static str;

    /// Priority (higher = try first)
    ///
    /// Standard formats should use 100, alternatives use 50.
    fn priority(&self) -> u32;
}

// ============================================================================
// Parser Registry
// ============================================================================

/// Registry of tool call parsers
///
/// Tries parsers in priority order (highest first) and returns the first
/// successful parse along with the parser name for debugging.
pub struct ToolCallParserRegistry {
    parsers: Vec<Box<dyn ToolCallParser>>,
}

impl Default for ToolCallParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolCallParserRegistry {
    /// Create a new registry with all built-in parsers
    pub fn new() -> Self {
        let mut parsers: Vec<Box<dyn ToolCallParser>> = vec![
            Box::new(StandardParser),
            Box::new(XmlFunctionParser::new()),
            Box::new(ToolArgsParser),
            Box::new(FunctionParamsParser),
        ];

        // Sort by priority (highest first)
        parsers.sort_by(|a, b| b.priority().cmp(&a.priority()));

        Self { parsers }
    }

    /// Try to parse content as a tool call
    ///
    /// Returns the parsed tool call and the name of the parser that succeeded,
    /// or None if no parser could handle the content.
    pub fn parse(&self, content: &str) -> Option<(ToolCall, &'static str)> {
        let content = content.trim();

        // Try each parser in priority order
        // Each parser handles its own format detection (JSON, XML, etc.)
        for parser in &self.parsers {
            if let Some(tool_call) = parser.parse(content) {
                return Some((tool_call, parser.name()));
            }
        }

        None
    }

    /// Get list of registered parser names (for debugging)
    pub fn parser_names(&self) -> Vec<&'static str> {
        self.parsers.iter().map(|p| p.name()).collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ToolCallParserRegistry::new();
        let names = registry.parser_names();

        assert!(names.contains(&"StandardParser"));
        assert!(names.contains(&"XmlFunctionParser"));
        assert!(names.contains(&"ToolArgsParser"));
        assert!(names.contains(&"FunctionParamsParser"));
    }

    #[test]
    fn test_registry_rejects_invalid() {
        let registry = ToolCallParserRegistry::new();

        assert!(registry.parse("hello world").is_none());
        assert!(registry.parse("").is_none());
        // Incomplete JSON still fails
        assert!(registry.parse("{ incomplete").is_none());
    }

    #[test]
    fn test_registry_xml_format() {
        let registry = ToolCallParserRegistry::new();

        let content = r#"<function=list_dir>
<parameter=path>/tmp</parameter>
</function>"#;
        let result = registry.parse(content);

        assert!(result.is_some());
        let (tool_call, parser_name) = result.unwrap();
        assert_eq!(tool_call.function.name, "list_dir");
        assert_eq!(parser_name, "XmlFunctionParser");
    }

    #[test]
    fn test_registry_standard_format() {
        let registry = ToolCallParserRegistry::new();

        let content = r#"{"name": "get_weather", "arguments": {"city": "NYC"}}"#;
        let result = registry.parse(content);

        assert!(result.is_some());
        let (tool_call, parser_name) = result.unwrap();
        assert_eq!(tool_call.function.name, "get_weather");
        assert_eq!(parser_name, "StandardParser");
    }
}
