//! Parameter types for GitHub MCP tools
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
