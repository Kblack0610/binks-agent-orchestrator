//! Web Search MCP Server
//!
//! Web search via SearXNG backend - self-hosted meta-search engine.
//!
//! # Configuration
//! Set `SEARXNG_URL` env var or configure in `~/.binks/web-search.toml`

mod backends;
mod config;
mod server;
mod types;

use server::WebSearchMcpServer;

mcp_common::serve_stdio!(WebSearchMcpServer, "web_search_mcp");
