//! Result helpers for MCP tool responses
//!
//! Provides convenient functions for creating `CallToolResult` responses,
//! reducing boilerplate in tool implementations.

use rmcp::{
    model::{CallToolResult, Content},
    ErrorData as McpError,
};
use serde::Serialize;

/// Create a successful JSON response from any serializable data
///
/// This replaces the common pattern:
/// ```rust,ignore
/// let json = serde_json::to_string_pretty(&data)
///     .map_err(|e| McpError::internal_error(e.to_string(), None))?;
/// Ok(CallToolResult::success(vec![Content::text(json)]))
/// ```
///
/// With simply:
/// ```rust,ignore
/// json_success(&data)
/// ```
///
/// # Arguments
///
/// * `data` - Any type that implements `Serialize`
///
/// # Returns
///
/// * `Ok(CallToolResult)` with pretty-printed JSON content
/// * `Err(McpError)` if serialization fails
///
/// # Example
///
/// ```rust,ignore
/// use mcp_common::json_success;
///
/// #[derive(Serialize)]
/// struct MyData { value: i32 }
///
/// fn my_tool(&self) -> Result<CallToolResult, McpError> {
///     let data = MyData { value: 42 };
///     json_success(&data)
/// }
/// ```
pub fn json_success<T: Serialize>(data: &T) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Create a successful plain text response
///
/// For tools that return simple text rather than structured data.
///
/// # Arguments
///
/// * `text` - Any type that can be converted to a `String`
///
/// # Example
///
/// ```rust,ignore
/// use mcp_common::text_success;
///
/// fn my_tool(&self) -> Result<CallToolResult, McpError> {
///     text_success("Operation completed successfully")
/// }
/// ```
pub fn text_success(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text.into())])
}

/// Create a successful response with multiple content items
///
/// For tools that need to return multiple pieces of content.
///
/// # Arguments
///
/// * `contents` - Vector of `Content` items
///
/// # Example
///
/// ```rust,ignore
/// use mcp_common::multi_success;
/// use rmcp::model::Content;
///
/// fn my_tool(&self) -> CallToolResult {
///     multi_success(vec![
///         Content::text("First part"),
///         Content::text("Second part"),
///     ])
/// }
/// ```
pub fn multi_success(contents: Vec<Content>) -> CallToolResult {
    CallToolResult::success(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_json_success() {
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };
        let result = json_success(&data).unwrap();
        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_text_success() {
        let result = text_success("hello world");
        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_multi_success() {
        let result = multi_success(vec![Content::text("a"), Content::text("b")]);
        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(result.content.len(), 2);
    }
}
