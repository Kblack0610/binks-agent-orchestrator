//! Handler implementations for Linear CLI MCP tools
//!
//! Organized by domain: issue, team, project, document

mod document;
mod issue;
mod project;
mod team;

pub use document::*;
pub use issue::*;
pub use project::*;
pub use team::*;

use mcp_common::{internal_error, McpError};

use crate::linear::LinearError;

/// Convert a LinearError to an MCP error
pub fn linear_to_mcp_error(e: LinearError) -> McpError {
    internal_error(e.to_string())
}
