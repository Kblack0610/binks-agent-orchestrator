//! Handler implementations for GitHub MCP tools
//!
//! Organized by domain: issue, pr, repo, workflow, search, release

mod issue;
mod pr;
mod release;
mod repo;
mod search;
mod workflow;

pub use issue::*;
pub use pr::*;
pub use release::*;
pub use repo::*;
pub use search::*;
pub use workflow::*;

use mcp_common::{internal_error, McpError};

use crate::gh::GhError;

/// Convert a GhError to an MCP error
pub fn gh_to_mcp_error(e: GhError) -> McpError {
    internal_error(e.to_string())
}
