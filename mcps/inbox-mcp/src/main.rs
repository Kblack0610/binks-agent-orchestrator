//! Inbox MCP Server
//!
//! This server provides a local file-based inbox for agent notifications.
//! Messages are written to `~/.notes/inbox/YYYY-MM-DD.md` files.
//!
//! # Features
//!
//! - **Write Inbox**: Add messages with timestamp, source, priority, and tags
//! - **Read Inbox**: Query recent messages with optional filters
//! - **Clear Inbox**: Archive old messages
//!
//! # Usage
//!
//! Configure in `.mcp.json`:
//! ```json
//! {
//!   "mcpServers": {
//!     "inbox": {
//!       "command": "./mcps/inbox-mcp/target/release/inbox-mcp",
//!       "env": {
//!         "INBOX_PATH": "${HOME}/.notes/inbox"
//!       }
//!     }
//!   }
//! }
//! ```

use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod server;
mod types;

use server::InboxMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (stdout is used for MCP protocol)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .with(EnvFilter::from_default_env().add_directive("inbox_mcp=info".parse()?))
        .init();

    tracing::info!("Starting Inbox MCP Server");

    // Create the MCP server
    let server = InboxMcpServer::new();

    // Create stdio transport and serve
    let service = server.serve(stdio()).await?;

    tracing::info!("Server running, waiting for requests...");

    // Wait for shutdown
    service.waiting().await?;

    tracing::info!("Server shutting down");
    Ok(())
}
