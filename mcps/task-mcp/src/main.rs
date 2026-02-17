//! Task MCP Server binary entry point

use rmcp::{transport::io::stdio, ServiceExt};
use task_mcp::TaskMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp_common::init_tracing("task_mcp")?;

    tracing::info!("Starting Task MCP server");

    let server = TaskMcpServer::new()?;
    let service = server.serve(stdio()).await?;

    tracing::info!("Task MCP server running");

    service.waiting().await?;

    tracing::info!("Task MCP server stopped");

    Ok(())
}
