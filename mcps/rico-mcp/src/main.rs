//! RICO MCP - Mobile UI design similarity search using the RICO dataset
//!
//! Provides access to 66,000+ Android UI screens with:
//! - 64-dimensional layout vectors for similarity search
//! - Semantic annotations (24 component types, 197 button concepts, 97 icon classes)
//! - Design pattern guidance and best practices

// Allow dead code for now - this is a new crate with planned features not yet wired up
#![allow(dead_code)]

mod config;
mod dataset;
mod params;
mod search;
mod server;
mod types;

use rmcp::{transport::io::stdio, ServiceExt};
use server::RicoMcpServer;

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
