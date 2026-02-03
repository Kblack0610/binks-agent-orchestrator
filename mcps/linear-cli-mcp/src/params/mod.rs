//! Parameter types for Linear CLI MCP tools

mod issue;
#[cfg(feature = "documents")]
mod document;

pub use issue::*;
#[cfg(feature = "documents")]
pub use document::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Empty parameters for tools that take no arguments
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EmptyParams {}
