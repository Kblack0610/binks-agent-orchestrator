//! Git MCP Library
//!
//! Local git operations using libgit2.
//! Provides git repository operations that complement GitHub API tools.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use git_mcp::GitMcpServer;
//!
//! let server = GitMcpServer::new();
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! Useful for local repo inspection, diffs, blame, and history analysis.

pub mod handlers;
pub mod params;
pub mod server;
pub mod types;

// Re-export main server type
pub use server::GitMcpServer;

// Re-export parameter types for direct API usage
pub use params::*;
