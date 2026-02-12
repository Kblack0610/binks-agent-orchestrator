//! GitHub CLI MCP Library
//!
//! MCP-compatible tools for GitHub via the `gh` CLI.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use github_gh_mcp::GitHubMcpServer;
//!
//! let server = GitHubMcpServer::new();
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! # Features
//! - Issues: List, view, create, edit, close
//! - Pull Requests: List, view, create, merge
//! - Workflows: List, trigger, view status
//! - Repositories: List and view
//!
//! # Requirements
//! - `gh` CLI installed and authenticated (`gh auth login`)

pub mod gh;
pub mod handlers;
pub mod params;
pub mod server;
pub mod tools;
pub mod types;

// Re-export main server type
pub use server::GitHubMcpServer;

// Re-export parameter types for direct API usage
pub use params::*;
