//! Self-Healing MCP Server binary entry point

use rmcp::{transport::io::stdio, ServiceExt};
use self_healing_mcp::SelfHealingMcpServer;

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
