//! Agent Registry MCP Library
//!
//! Service discovery and resource coordination for concurrent agents.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use agent_registry_mcp::AgentRegistryMcpServer;
//!
//! let server = AgentRegistryMcpServer::new()?;
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! - Agents register on startup and heartbeat for liveness
//! - Ports and resources can be claimed to avoid conflicts
//! - Shares ~/.binks/conversations.db with task-mcp and the agent

pub mod handlers;
pub mod params;
pub mod repository;
pub mod schema;
pub mod server;
#[cfg(test)]
pub mod tests;
pub mod types;

// Re-export main server type
pub use server::AgentRegistryMcpServer;

// Re-export parameter types for direct API usage
pub use params::*;
