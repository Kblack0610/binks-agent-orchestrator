//! Embeddable MCP trait for in-process execution
//!
//! This module provides the [`EmbeddableMcp`] trait that allows MCP servers
//! to be executed directly in-process without subprocess spawning or IPC.
//!
//! # Example
//!
//! ```rust,ignore
//! use mcp_common::EmbeddableMcp;
//! use sysinfo_mcp::SysInfoMcpServer;
//!
//! let server = SysInfoMcpServer::new();
//!
//! // List available tools
//! let tools = server.list_tools();
//! println!("Available: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());
//!
//! // Call a tool directly
//! let result = server.call_tool("get_cpu_info", serde_json::json!({})).await?;
//! ```

use async_trait::async_trait;
use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;

/// Error type for embeddable MCP operations
#[derive(Debug, thiserror::Error)]
pub enum EmbeddableError {
    /// Tool was not found in the server
    #[error("tool not found: {0}")]
    ToolNotFound(String),

    /// Invalid parameters passed to the tool
    #[error("invalid parameters: {0}")]
    InvalidParams(String),

    /// Tool execution failed
    #[error("tool execution failed: {0}")]
    ExecutionError(String),

    /// Serialization/deserialization error
    #[error("serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    /// MCP protocol error
    #[error("mcp error: {0}")]
    McpError(String),
}

impl From<rmcp::ErrorData> for EmbeddableError {
    fn from(err: rmcp::ErrorData) -> Self {
        EmbeddableError::McpError(err.message.to_string())
    }
}

/// Result type for embeddable MCP operations
pub type EmbeddableResult<T> = Result<T, EmbeddableError>;

/// Trait for MCP servers that can be executed in-process
///
/// This trait enables direct execution of MCP servers without subprocess
/// spawning or IPC. Servers implementing this trait can be embedded directly
/// into a host application for lower latency and tighter integration.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to support concurrent tool calls
/// from multiple async tasks.
///
/// # Implementation
///
/// The trait is designed to work with rmcp's `ToolRouter<Self>` pattern.
/// Servers that use `#[tool_router]` can implement this trait by delegating
/// to their internal router.
///
/// ```rust,ignore
/// #[async_trait]
/// impl EmbeddableMcp for MySever {
///     fn server_name(&self) -> &str {
///         "my-server"
///     }
///
///     fn list_tools(&self) -> Vec<Tool> {
///         self.tool_router.list_all()
///     }
///
///     async fn call_tool(&self, name: &str, params: Value) -> EmbeddableResult<CallToolResult> {
///         // Delegate to tool router with proper context
///         // ...
///     }
/// }
/// ```
#[async_trait]
pub trait EmbeddableMcp: Send + Sync {
    /// Returns the server name for identification
    ///
    /// This should match the server name used in MCP configuration files.
    fn server_name(&self) -> &str;

    /// Returns a list of all available tools
    ///
    /// Each tool includes its name, description, and input schema.
    fn list_tools(&self) -> Vec<Tool>;

    /// Executes a tool by name with the given parameters
    ///
    /// # Arguments
    ///
    /// * `name` - The tool name as returned by `list_tools`
    /// * `params` - JSON object containing the tool parameters
    ///
    /// # Returns
    ///
    /// Returns the tool result on success, or an error if:
    /// - The tool is not found
    /// - The parameters are invalid
    /// - Tool execution fails
    async fn call_tool(&self, name: &str, params: Value) -> EmbeddableResult<CallToolResult>;

    /// Returns an optional description of the server
    ///
    /// This is used for documentation and discovery purposes.
    fn server_description(&self) -> Option<&str> {
        None
    }

    /// Returns the server version, if available
    fn server_version(&self) -> Option<&str> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal test implementation
    struct TestServer;

    #[async_trait]
    impl EmbeddableMcp for TestServer {
        fn server_name(&self) -> &str {
            "test-server"
        }

        fn list_tools(&self) -> Vec<Tool> {
            vec![]
        }

        async fn call_tool(&self, name: &str, _params: Value) -> EmbeddableResult<CallToolResult> {
            Err(EmbeddableError::ToolNotFound(name.to_string()))
        }
    }

    #[test]
    fn test_server_name() {
        let server = TestServer;
        assert_eq!(server.server_name(), "test-server");
    }

    #[test]
    fn test_list_tools_empty() {
        let server = TestServer;
        assert!(server.list_tools().is_empty());
    }

    #[tokio::test]
    async fn test_call_unknown_tool() {
        let server = TestServer;
        let result = server.call_tool("unknown", serde_json::json!({})).await;
        assert!(matches!(result, Err(EmbeddableError::ToolNotFound(_))));
    }
}
