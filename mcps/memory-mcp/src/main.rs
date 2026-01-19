//! Memory MCP - Dual-layer memory server with session and persistent storage
//!
//! Session layer: In-memory, ephemeral storage for reasoning chains and working memory
//! Persistent layer: SQLite-backed knowledge graph that survives across sessions

mod persistent;
mod server;
mod session;
mod types;

use rmcp::{transport::io::stdio, ServiceExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use server::MemoryMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (MCP uses stdio for protocol)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .with(EnvFilter::from_default_env().add_directive("memory_mcp=info".parse()?))
        .init();

    tracing::info!("Starting Memory MCP server");

    // Create the server
    let server = MemoryMcpServer::new()?;

    // Start serving on stdio
    let service = server.serve(stdio()).await?;

    tracing::info!("Memory MCP server running");

    // Wait for shutdown
    service.waiting().await?;

    tracing::info!("Memory MCP server stopped");

    Ok(())
}
