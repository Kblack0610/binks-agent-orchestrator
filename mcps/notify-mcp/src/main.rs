//! Notify MCP Server
//!
//! Notification capabilities via Slack and Discord webhooks.
//!
//! # Configuration
//! Set `SLACK_WEBHOOK_URL` and/or `DISCORD_WEBHOOK_URL` env vars.

mod server;

use server::NotifyMcpServer;

mcp_common::serve_stdio!(NotifyMcpServer, "notify_mcp");
