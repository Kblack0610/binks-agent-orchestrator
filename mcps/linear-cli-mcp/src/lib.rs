//! Linear CLI MCP Library
//!
//! MCP-compatible tools for Linear issue tracking via the `linear` CLI.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use linear_cli_mcp::LinearCliMcpServer;
//!
//! let server = LinearCliMcpServer::new();
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! # Features
//! - Issues: List, view, create, start, comment, get ID from branch
//! - Teams: List, members
//! - Projects: List
//! - Documents: List, view
//!
//! # Requirements
//! - `linear` CLI installed (`brew install schpet/tap/linear`)

pub mod handlers;
pub mod linear;
pub mod params;
pub mod server;

// Re-export main server type
pub use server::LinearCliMcpServer;

// Re-export parameter types for direct API usage
pub use params::*;
