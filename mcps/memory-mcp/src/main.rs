//! Memory MCP Server binary entry point

use memory_mcp::MemoryMcpServer;
use rmcp::{transport::io::stdio, ServiceExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp_common::init_tracing("memory_mcp")?;

    tracing::info!("Starting Memory MCP server");

    let server = MemoryMcpServer::new()?;
    let service = server.serve(stdio()).await?;

    tracing::info!("Memory MCP server running");

    service.waiting().await?;

    tracing::info!("Memory MCP server stopped");

    Ok(())
}
