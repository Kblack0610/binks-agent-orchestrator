//! Notify MCP Library
//!
//! Notification capabilities via Slack and Discord webhooks.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use notify_mcp::NotifyMcpServer;
//!
//! let server = NotifyMcpServer::new();
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! # Configuration
//! Set `SLACK_WEBHOOK_URL` and/or `DISCORD_WEBHOOK_URL` env vars.

pub mod server;

// Re-export main server type
pub use server::NotifyMcpServer;

// Re-export parameter types for direct API usage
pub use server::{DigestParams, DiscordMessageParams, SlackMessageParams};
