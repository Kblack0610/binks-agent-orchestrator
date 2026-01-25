//! Git MCP - Local git operations server using libgit2
//!
//! Provides git repository operations that complement GitHub API tools.
//! Useful for local repo inspection, diffs, blame, and history analysis.
//!
//! # Usage
//!
//! Run directly: `git-mcp`
//!
//! Or configure in `.mcp.json`:
//! ```json
//! { "mcpServers": { "git": { "command": "./git-mcp" } } }
//! ```

mod handlers;
mod params;
mod server;
mod types;

use server::GitMcpServer;

mcp_common::serve_stdio!(GitMcpServer, "git_mcp");
