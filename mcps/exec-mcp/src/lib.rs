//! Exec MCP Library
//!
//! Sandboxed command execution server with security controls.
//! Provides shell command execution with configurable allow/deny lists,
//! timeout enforcement, and output size limits.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use exec_mcp::ExecMcpServer;
//!
//! let server = ExecMcpServer::new();
//! // Use with in-memory transport or serve via stdio
//! ```

pub mod guard;
pub mod handlers;
pub mod params;
pub mod server;
pub mod types;

// Re-export main server type
pub use server::ExecMcpServer;

// Re-export parameter types for direct API usage
pub use params::*;
