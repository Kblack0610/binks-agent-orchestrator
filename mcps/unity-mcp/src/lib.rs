//! Unity MCP Library
//!
//! Filesystem-based Unity Editor log monitoring and project analysis via MCP.
//! Reads Unity state entirely from the filesystem — no Editor plugin required.

pub mod detect;
pub mod handlers;
pub mod log_parser;
pub mod params;
pub mod server;

pub use server::UnityMcpServer;
pub use mcp_common::{EmbeddableError, EmbeddableMcp, EmbeddableResult};
