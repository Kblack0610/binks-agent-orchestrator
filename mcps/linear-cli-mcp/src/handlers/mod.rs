//! Handler implementations for Linear CLI MCP tools
//!
//! Organized by domain: issue, team, project, document

mod issue;
mod team;
mod project;
mod document;

pub use issue::*;
pub use team::*;
pub use project::*;
pub use document::*;

use mcp_common::{internal_error, McpError};

use crate::linear::LinearError;

/// Convert a LinearError to an MCP error
pub fn linear_to_mcp_error(e: LinearError) -> McpError {
    internal_error(e.to_string())
}
