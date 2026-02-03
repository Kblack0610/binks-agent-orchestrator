//! Handler implementations for Linear CLI MCP tools
//!
//! Organized by domain: issue, team, project, document

mod issue;
#[cfg(feature = "teams")]
mod team;
#[cfg(feature = "projects")]
mod project;
#[cfg(feature = "documents")]
mod document;

pub use issue::*;
#[cfg(feature = "teams")]
pub use team::*;
#[cfg(feature = "projects")]
pub use project::*;
#[cfg(feature = "documents")]
pub use document::*;

use mcp_common::{internal_error, McpError};

use crate::linear::LinearError;

/// Convert a LinearError to an MCP error
pub fn linear_to_mcp_error(e: LinearError) -> McpError {
    internal_error(e.to_string())
}
