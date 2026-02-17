//! Web Search MCP Library
//!
//! Web search via SearXNG backend - self-hosted meta-search engine.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use web_search_mcp::WebSearchMcpServer;
//!
//! let server = WebSearchMcpServer::new();
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! # Configuration
//! Set `SEARXNG_URL` env var or configure in `~/.binks/web-search.toml`

pub mod backends;
pub mod config;
pub mod fetch;
pub mod server;
pub mod types;

// Re-export main server type
pub use server::WebSearchMcpServer;

// Re-export parameter types for direct API usage
pub use server::{
    FetchJsonParams, FetchMarkdownParams, FetchParams, ImageSearchParams, NewsSearchParams,
    ParseHtmlParams, SearchParams,
};
