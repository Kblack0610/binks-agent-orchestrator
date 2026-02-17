//! Memory MCP Library
//!
//! Dual-layer memory server with session and persistent storage.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use memory_mcp::MemoryMcpServer;
//!
//! let server = MemoryMcpServer::new()?;
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! - Session layer: In-memory, ephemeral storage for reasoning chains and working memory
//! - Persistent layer: SQLite-backed knowledge graph that survives across sessions

pub mod handlers;
pub mod params;
pub mod persistent;
pub mod server;
pub mod session;
pub mod types;

// Re-export main server type
pub use server::MemoryMcpServer;

// Re-export parameter types for direct API usage
pub use params::*;
