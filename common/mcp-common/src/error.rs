//! Error handling utilities for MCP servers
//!
//! Provides traits and types for consistent error handling across MCP servers.

use rmcp::ErrorData as McpError;

/// Type alias for MCP tool results
pub type McpResult<T> = Result<T, McpError>;

/// Trait for converting errors into MCP-compatible errors
///
/// Implement this trait for external error types to enable the `?` operator
/// in tool implementations.
///
/// # Example
///
/// ```rust,ignore
/// use mcp_common::IntoMcpError;
/// use rmcp::ErrorData as McpError;
///
/// // For a custom error type
/// impl IntoMcpError for MyError {
///     fn into_mcp_error(self) -> McpError {
///         McpError::internal_error(self.to_string(), None)
///     }
/// }
///
/// // Or implement From<MyError> for McpError using the trait
/// impl From<MyError> for McpError {
///     fn from(e: MyError) -> Self {
///         e.into_mcp_error()
///     }
/// }
/// ```
pub trait IntoMcpError {
    /// Convert this error into an MCP error
    fn into_mcp_error(self) -> McpError;
}

// Implement for common error types

impl IntoMcpError for std::io::Error {
    fn into_mcp_error(self) -> McpError {
        McpError::internal_error(format!("IO error: {}", self), None)
    }
}

impl IntoMcpError for serde_json::Error {
    fn into_mcp_error(self) -> McpError {
        McpError::internal_error(format!("JSON error: {}", self), None)
    }
}

impl IntoMcpError for anyhow::Error {
    fn into_mcp_error(self) -> McpError {
        McpError::internal_error(self.to_string(), None)
    }
}

impl IntoMcpError for String {
    fn into_mcp_error(self) -> McpError {
        McpError::internal_error(self, None)
    }
}

impl IntoMcpError for &str {
    fn into_mcp_error(self) -> McpError {
        McpError::internal_error(self.to_string(), None)
    }
}

/// Extension trait for Result types to convert to MCP errors
///
/// Provides a convenient `to_mcp_err()` method for any Result where
/// the error type implements `IntoMcpError`.
///
/// # Example
///
/// ```rust,ignore
/// use mcp_common::ResultExt;
///
/// fn my_tool(&self) -> Result<CallToolResult, McpError> {
///     let data = std::fs::read_to_string("file.txt").to_mcp_err()?;
///     // ...
/// }
/// ```
pub trait ResultExt<T> {
    /// Convert the error to an MCP error
    fn to_mcp_err(self) -> Result<T, McpError>;
}

impl<T, E: IntoMcpError> ResultExt<T> for Result<T, E> {
    fn to_mcp_err(self) -> Result<T, McpError> {
        self.map_err(|e| e.into_mcp_error())
    }
}

/// Create an internal error with a message
///
/// Convenience function for creating MCP internal errors.
///
/// # Example
///
/// ```rust,ignore
/// use mcp_common::internal_error;
///
/// fn my_tool(&self) -> Result<CallToolResult, McpError> {
///     if bad_condition {
///         return Err(internal_error("Something went wrong"));
///     }
///     // ...
/// }
/// ```
pub fn internal_error(message: impl Into<String>) -> McpError {
    McpError::internal_error(message.into(), None)
}

/// Create an invalid params error with a message
///
/// Use this when the tool receives invalid parameters.
///
/// # Example
///
/// ```rust,ignore
/// use mcp_common::invalid_params;
///
/// fn my_tool(&self, path: &str) -> Result<CallToolResult, McpError> {
///     if path.is_empty() {
///         return Err(invalid_params("path cannot be empty"));
///     }
///     // ...
/// }
/// ```
pub fn invalid_params(message: impl Into<String>) -> McpError {
    McpError::invalid_params(message.into(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_mcp_error_string() {
        let err = "test error".into_mcp_error();
        assert!(err.message.contains("test error"));
    }

    #[test]
    fn test_result_ext() {
        let result: Result<(), std::io::Error> =
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "not found"));
        let mcp_result = result.to_mcp_err();
        assert!(mcp_result.is_err());
    }

    #[test]
    fn test_internal_error() {
        let err = internal_error("test");
        assert!(err.message.contains("test"));
    }

    #[test]
    fn test_invalid_params() {
        let err = invalid_params("bad param");
        assert!(err.message.contains("bad param"));
    }
}
