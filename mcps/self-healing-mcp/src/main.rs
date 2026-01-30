//! Self-Healing MCP - Workflow health analysis and automated improvement proposals
//!
//! Analyzes run history from ~/.binks/conversations.db to detect patterns,
//! propose fixes, and verify improvements. Integrates with inbox-mcp for notifications.

mod analysis;
mod handlers;
mod params;
mod server;
mod strategies;
mod types;

use rmcp::{transport::io::stdio, ServiceExt};
use server::SelfHealingMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp_common::init_tracing("self_healing_mcp")?;

    tracing::info!("Starting Self-Healing MCP server");

    let server = SelfHealingMcpServer::new()?;
    let service = server.serve(stdio()).await?;

    tracing::info!("Self-Healing MCP server running");

    service.waiting().await?;

    tracing::info!("Self-Healing MCP server stopped");

    Ok(())
}
