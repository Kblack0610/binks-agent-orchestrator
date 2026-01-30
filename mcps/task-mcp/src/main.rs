//! Task MCP - Task management with CRUD, dependencies, and execution tracking
//!
//! Shares ~/.binks/conversations.db with the agent for task execution state.
//! Integrates with memory-mcp for task knowledge and context.

mod handlers;
mod params;
mod repository;
mod schema;
mod server;
#[cfg(test)]
mod tests;
mod types;

use rmcp::{transport::io::stdio, ServiceExt};
use server::TaskMcpServer;

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
