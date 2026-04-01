//! Agent Registry MCP Server binary entry point

use agent_registry_mcp::AgentRegistryMcpServer;
use rmcp::{transport::io::stdio, ServiceExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp_common::init_tracing("agent_registry_mcp")?;

    tracing::info!("Starting Agent Registry MCP server");

    let server = AgentRegistryMcpServer::new()?;
    let service = server.serve(stdio()).await?;

    tracing::info!("Agent Registry MCP server running");

    service.waiting().await?;

    tracing::info!("Agent Registry MCP server stopped");

    Ok(())
}
