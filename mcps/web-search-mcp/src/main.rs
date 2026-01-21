//! Web Search MCP Server
//!
//! This server provides web search capabilities via the Model Context Protocol (MCP).
//! It uses SearXNG as the search backend - a self-hosted meta-search engine.
//!
//! # Features
//!
//! - **SearXNG Backend**: Self-hosted, no API keys, aggregates multiple search engines
//! - **Web Search**: General web search with configurable result limits
//! - **News Search**: News-specific search results
//! - **Image Search**: Image search from multiple sources
//!
//! # Configuration
//!
//! Configure via environment variable or `~/.binks/web-search.toml`:
//!
//! ```bash
//! export SEARXNG_URL="http://searxng-service:8080"
//! ```
//!
//! Or via config file:
//! ```toml
//! [search]
//! max_results = 10
//!
//! [searxng]
//! url = "http://localhost:8080"
//! ```
//!
//! # Usage
//!
//! Run directly:
//! ```bash
//! web-search-mcp
//! ```
//!
//! Or configure in `.mcp.json`:
//! ```json
//! {
//!   "mcpServers": {
//!     "web-search": {
//!       "command": "./mcps/web-search-mcp/target/release/web-search-mcp",
//!       "env": {
//!         "SEARXNG_URL": "http://searxng-service:8080"
//!       }
//!     }
//!   }
//! }
//! ```

use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod backends;
mod config;
mod server;
mod types;

use config::Config;
use server::WebSearchMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (stdout is used for MCP protocol)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .with(EnvFilter::from_default_env().add_directive("web_search_mcp=info".parse()?))
        .init();

    tracing::info!("Starting Web Search MCP Server");

    // Load configuration
    let config = Config::load()?;
    tracing::info!("SearXNG URL: {}", config.searxng.url);

    // Create the MCP server with SearXNG backend
    let server = WebSearchMcpServer::new(config).await?;

    // Create stdio transport and serve
    let service = server.serve(stdio()).await?;

    tracing::info!("Server running, waiting for requests...");

    // Wait for shutdown
    service.waiting().await?;

    tracing::info!("Server shutting down");
    Ok(())
}
