//! SQL MCP Library
//!
//! Database query tools for SQL databases.
//! Read-only by default, write mode can be enabled via config.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use sql_mcp::SqlMcpServer;
//!
//! let server = SqlMcpServer::new();
//! // Use with in-memory transport or serve via stdio
//! ```

pub mod config;
pub mod server;

// Re-export main server type
pub use server::SqlMcpServer;

// Re-export parameter types for direct API usage
pub use server::{QueryParams, TablesParams};
