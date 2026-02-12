//! Self-Healing MCP Library
//!
//! Workflow health analysis and automated improvement proposals.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use self_healing_mcp::SelfHealingMcpServer;
//!
//! let server = SelfHealingMcpServer::new()?;
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! - Analyzes run history from ~/.binks/conversations.db
//! - Detects failure patterns and proposes improvements
//! - Verifies improvement impact over measurement windows
//! - Integrates with inbox-mcp for notifications

pub mod analysis;
pub mod handlers;
pub mod inbox;
pub mod params;
pub mod server;
pub mod strategies;
pub mod types;

// Re-export main server type
pub use server::SelfHealingMcpServer;

// Re-export parameter types for direct API usage
pub use params::*;
