//! RICO MCP Library
//!
//! Mobile UI design similarity search using the RICO dataset.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use rico_mcp::RicoMcpServer;
//!
//! let server = RicoMcpServer::new()?;
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! - Provides access to 66,000+ Android UI screens
//! - 64-dimensional layout vectors for similarity search
//! - Semantic annotations (24 component types, 197 button concepts, 97 icon classes)
//! - Design pattern guidance and best practices

// Allow dead code for now - this is a new crate with planned features not yet wired up
#![allow(dead_code)]

pub mod config;
pub mod dataset;
pub mod params;
pub mod search;
pub mod server;
pub mod types;

// Re-export main server type
pub use server::RicoMcpServer;

// Re-export parameter types for direct API usage
pub use params::*;
