//! Parameter types for Linear CLI MCP tools

mod document;
mod issue;

pub use document::*;
pub use issue::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Empty parameters for tools that take no arguments
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EmptyParams {}
