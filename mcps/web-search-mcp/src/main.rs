//! Web Search MCP Server
//!
//! Web search via SearXNG backend - self-hosted meta-search engine.
//!
//! # Configuration
//! Set `SEARXNG_URL` env var or configure in `~/.binks/web-search.toml`

use rmcp::{transport::stdio, ServiceExt};

mod backends;
mod config;
mod server;
mod types;

use config::Config;
use server::WebSearchMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp_common::init_tracing("web_search_mcp")?;

    tracing::info!("Starting Web Search MCP Server");

    let config = Config::load()?;
    tracing::info!("SearXNG URL: {}", config.searxng.url);

    let server = WebSearchMcpServer::new(config).await?;
    let service = server.serve(stdio()).await?;

    tracing::info!("Server running, waiting for requests...");
    service.waiting().await?;

    tracing::info!("Server shutting down");
    Ok(())
}
