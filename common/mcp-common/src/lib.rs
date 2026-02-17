//! MCP Common - Shared utilities for MCP servers
//!
//! This crate provides common functionality used across all MCP servers:
//!
//! - **Initialization**: `serve_stdio!` macro for standardized server startup
//! - **Results**: Helper functions for creating `CallToolResult` responses
//! - **Errors**: Traits for converting errors to MCP-compatible format
//! - **Embeddable**: [`EmbeddableMcp`] trait for in-process execution
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
//!
//! # Embedding MCPs
//!
//! For in-process execution without subprocess spawning:
//!
//! ```rust,ignore
//! use mcp_common::EmbeddableMcp;
//! use sysinfo_mcp::SysInfoMcpServer;
//!
//! let server = SysInfoMcpServer::new();
//! let tools = server.list_tools();
//! let result = server.call_tool("get_cpu_info", serde_json::json!({})).await?;
//! ```

pub mod embeddable;
pub mod error;
pub mod init;
pub mod result;

// Re-export commonly used items at crate root
pub use embeddable::{EmbeddableError, EmbeddableMcp, EmbeddableResult};
pub use error::{internal_error, invalid_params, IntoMcpError, McpResult, ResultExt};
pub use init::init_tracing;
pub use result::{json_success, text_success};

// Re-export rmcp types that are commonly needed
pub use rmcp::{
    model::{CallToolResult, Content, Tool},
    ErrorData as McpError,
};

// Re-export async_trait for implementing EmbeddableMcp
pub use async_trait::async_trait;
