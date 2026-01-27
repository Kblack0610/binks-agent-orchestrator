//! GitHub CLI MCP Server
//!
//! MCP-compatible tools for GitHub via the `gh` CLI.
//!
//! # Features
//! - Issues: List, view, create, edit, close
//! - Pull Requests: List, view, create, merge
//! - Workflows: List, trigger, view status
//! - Repositories: List and view
//!
//! # Requirements
//! - `gh` CLI installed and authenticated (`gh auth login`)

mod gh;
mod handlers;
mod params;
mod server;
mod tools;
mod types;

use server::GitHubMcpServer;

mcp_common::serve_stdio!(GitHubMcpServer, "github_gh_mcp");
