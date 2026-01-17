//! Notify MCP Server
//!
//! This server provides notification capabilities via Slack and Discord webhooks.
//!
//! # Features
//!
//! - **Slack**: Send messages via webhook URL
//! - **Discord**: Send messages via webhook URL
//! - **Digest**: Send formatted daily digests
//!
//! # Configuration
//!
//! Set environment variables:
//! - `SLACK_WEBHOOK_URL`: Slack incoming webhook URL
//! - `DISCORD_WEBHOOK_URL`: Discord webhook URL
//!
//! # Usage
//!
//! Configure in `.mcp.json`:
//! ```json
//! {
//!   "mcpServers": {
//!     "notify": {
//!       "command": "./mcps/notify-mcp/target/release/notify-mcp",
//!       "env": {
//!         "SLACK_WEBHOOK_URL": "${SLACK_WEBHOOK_URL}",
//!         "DISCORD_WEBHOOK_URL": "${DISCORD_WEBHOOK_URL}"
//!       }
//!     }
//!   }
//! }
//! ```

use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod server;

use server::NotifyMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (stdout is used for MCP protocol)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .with(EnvFilter::from_default_env().add_directive("notify_mcp=info".parse()?))
        .init();

    tracing::info!("Starting Notify MCP Server");

    // Create the MCP server
    let server = NotifyMcpServer::new();

    // Create stdio transport and serve
    let service = server.serve(stdio()).await?;

    tracing::info!("Server running, waiting for requests...");

    // Wait for shutdown
    service.waiting().await?;

    tracing::info!("Server shutting down");
    Ok(())
}
