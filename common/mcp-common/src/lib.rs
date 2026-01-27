//! MCP Common - Shared utilities for MCP servers
//!
//! This crate provides common functionality used across all MCP servers:
//!
//! - **Initialization**: `serve_stdio!` macro for standardized server startup
//! - **Results**: Helper functions for creating `CallToolResult` responses
//! - **Errors**: Traits for converting errors to MCP-compatible format
//!
//! # Example
//!
//! ```rust,ignore
//! use mcp_common::{serve_stdio, json_success};
//! use rmcp::model::CallToolResult;
//!
//! // In main.rs - replaces ~30 lines of boilerplate
//! serve_stdio!(MyServer, "my-mcp");
//!
//! // In tool implementations - replaces 3-4 lines each
//! fn my_tool(&self) -> Result<CallToolResult, McpError> {
//!     let data = get_some_data();
//!     json_success(&data)
//! }
//! ```

pub mod error;
pub mod init;
pub mod result;

// Re-export commonly used items at crate root
pub use error::{internal_error, invalid_params, IntoMcpError, McpResult, ResultExt};
pub use init::init_tracing;
pub use result::{json_success, text_success};

// Re-export rmcp types that are commonly needed
pub use rmcp::{
    model::{CallToolResult, Content},
    ErrorData as McpError,
};
