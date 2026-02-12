//! Inbox MCP Library
//!
//! Local file-based inbox for agent notifications.
//! Messages written to `~/.notes/inbox/YYYY-MM-DD.md` files.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use inbox_mcp::InboxMcpServer;
//!
//! let server = InboxMcpServer::new();
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! # Features
//! - Write messages with timestamp, source, priority, tags
//! - Query recent messages with filters
//! - Archive old messages

pub mod server;
pub mod types;

// Re-export main server type
pub use server::InboxMcpServer;

// Re-export parameter types for direct API usage
pub use server::{ClearInboxParams, ReadInboxParams, WriteInboxParams};
