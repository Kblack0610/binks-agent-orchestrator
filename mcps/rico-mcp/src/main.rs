//! RICO MCP Server binary entry point

use rico_mcp::RicoMcpServer;
use rmcp::{transport::io::stdio, ServiceExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp_common::init_tracing("rico_mcp")?;

    tracing::info!("Starting RICO MCP server");

    let server = RicoMcpServer::new()?;
    let service = server.serve(stdio()).await?;

    tracing::info!("RICO MCP server running");

    service.waiting().await?;

    tracing::info!("RICO MCP server stopped");

    Ok(())
}
