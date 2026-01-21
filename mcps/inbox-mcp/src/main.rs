//! Inbox MCP Server
//!
//! Local file-based inbox for agent notifications.
//! Messages written to `~/.notes/inbox/YYYY-MM-DD.md` files.
//!
//! # Features
//! - Write messages with timestamp, source, priority, tags
//! - Query recent messages with filters
//! - Archive old messages

mod server;
mod types;

use server::InboxMcpServer;

mcp_common::serve_stdio!(InboxMcpServer, "inbox_mcp");
