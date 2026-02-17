//! Filesystem MCP Library
//!
//! Sandboxed filesystem server with security controls.
//! Provides secure file operations with configurable allowlists/denylists.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use filesystem_mcp::FilesystemMcpServer;
//!
//! let server = FilesystemMcpServer::new();
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! Operations are restricted to configured directories.

pub mod handlers;
pub mod params;
pub mod sandbox;
pub mod server;
pub mod types;

// Re-export main server type
pub use server::FilesystemMcpServer;

// Re-export parameter types for direct API usage
pub use params::*;
