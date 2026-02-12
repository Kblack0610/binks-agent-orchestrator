//! Task MCP Library
//!
//! Task management with CRUD operations, dependency tracking, and execution state.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use task_mcp::TaskMcpServer;
//!
//! let server = TaskMcpServer::new()?;
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! - Shares ~/.binks/conversations.db with the agent for task execution state
//! - Supports task dependencies with blocking/blocked relationships
//! - Integrates with memory-mcp for task knowledge and context

pub mod handlers;
pub mod params;
pub mod repository;
pub mod schema;
pub mod server;
#[cfg(test)]
pub mod tests;
pub mod types;

// Re-export main server type
pub use server::TaskMcpServer;

// Re-export parameter types for direct API usage
pub use params::*;
