//! Memory MCP - Dual-layer memory server with session and persistent storage
//!
//! Session layer: In-memory, ephemeral storage for reasoning chains and working memory
//! Persistent layer: SQLite-backed knowledge graph that survives across sessions

mod persistent;
mod server;
mod session;
mod types;

use rmcp::{transport::io::stdio, ServiceExt};
use server::MemoryMcpServer;

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
