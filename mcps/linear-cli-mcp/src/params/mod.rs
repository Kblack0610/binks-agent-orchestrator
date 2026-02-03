//! Parameter types for Linear CLI MCP tools

mod issue;
mod document;

pub use issue::*;
pub use document::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Empty parameters for tools that take no arguments
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EmptyParams {}
