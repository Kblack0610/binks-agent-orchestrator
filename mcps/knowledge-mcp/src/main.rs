//! Knowledge MCP Server binary entry point

use knowledge_mcp::config::KnowledgeConfig;
use knowledge_mcp::KnowledgeMcpServer;
use rmcp::{transport::io::stdio, ServiceExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp_common::init_tracing("knowledge_mcp")?;

    tracing::info!("Starting Knowledge MCP server");

    let config = KnowledgeConfig::load().map_err(|e| {
        eprintln!("Error: {e}");
        e
    })?;

    tracing::info!(db_path = %config.db_path().display(), sources = config.sources.len(), "Config loaded");

    let server = KnowledgeMcpServer::new(config)?;
    let service = server.serve(stdio()).await?;

    tracing::info!("Knowledge MCP server running");

    service.waiting().await?;

    tracing::info!("Knowledge MCP server stopped");

    Ok(())
}
