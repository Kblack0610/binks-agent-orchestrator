//! MCP Server implementation for web search
//!
//! This module defines the main MCP server that exposes web search
//! tools with pluggable backend support.

use mcp_common::{json_success, CallToolResult, McpError, ResultExt};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::backends::{searxng::SearXNGBackend, SearchBackend};
use crate::config::Config;

/// The main Web Search MCP Server
#[derive(Clone)]
pub struct WebSearchMcpServer {
    backend: Arc<dyn SearchBackend>,
    config: Config,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchParams {
    /// The search query
    #[schemars(description = "The search query string")]
    pub query: String,
    /// Maximum number of results to return
    #[schemars(description = "Maximum number of results to return (default: 10)")]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NewsSearchParams {
    /// The search query
    #[schemars(description = "The news search query string")]
    pub query: String,
    /// Maximum number of results to return
    #[schemars(description = "Maximum number of results to return (default: 10)")]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ImageSearchParams {
    /// The search query
    #[schemars(description = "The image search query string")]
    pub query: String,
    /// Maximum number of results to return
    #[schemars(description = "Maximum number of results to return (default: 10)")]
    pub limit: Option<usize>,
}

// ============================================================================
// Tool Router Implementation
// ============================================================================

#[tool_router]
impl WebSearchMcpServer {
    /// Create a new server, loading config from standard locations
    ///
    /// Config is loaded from:
    /// 1. Environment variable SEARXNG_URL (highest priority for URL)
    /// 2. Environment variable WEB_SEARCH_CONFIG_PATH
    /// 3. ~/.binks/web-search.toml
    /// 4. Default values
    pub fn new() -> Self {
        let config = Config::load().expect("Failed to load web-search configuration");
        Self::with_config(config)
    }

    /// Create a new server with explicit config
    pub fn with_config(config: Config) -> Self {
        // SearXNG is the only supported backend (self-hosted, no API keys needed)
        tracing::info!("Using SearXNG backend at {}", config.searxng.url);
        let backend: Arc<dyn SearchBackend> = Arc::new(SearXNGBackend::new(config.searxng.clone()));

        if !backend.is_available() {
            tracing::warn!(
                "Backend '{}' is not available (check SearXNG URL)",
                backend.name()
            );
        }

        Self {
            backend,
            config,
            tool_router: Self::tool_router(),
        }
    }

    // ========================================================================
    // Search Tools
    // ========================================================================

    #[tool(description = "Search the web for information. Returns titles, URLs, and descriptions.")]
    async fn search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(self.config.search.max_results);

        tracing::info!("Searching for: {} (limit: {})", params.query, limit);

        let results = self
            .backend
            .search(&params.query, limit)
            .await
            .to_mcp_err()?;

        json_success(&results)
    }

    #[tool(
        description = "Search for news articles. Returns titles, URLs, sources, and publication dates."
    )]
    async fn search_news(
        &self,
        Parameters(params): Parameters<NewsSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(self.config.search.max_results);

        tracing::info!("Searching news for: {} (limit: {})", params.query, limit);

        let results = self
            .backend
            .search_news(&params.query, limit)
            .await
            .to_mcp_err()?;

        json_success(&results)
    }

    #[tool(description = "Search for images. Returns image URLs, page URLs, and dimensions.")]
    async fn search_images(
        &self,
        Parameters(params): Parameters<ImageSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(self.config.search.max_results);

        tracing::info!("Searching images for: {} (limit: {})", params.query, limit);

        let results = self
            .backend
            .search_images(&params.query, limit)
            .await
            .to_mcp_err()?;

        json_success(&results)
    }

    #[tool(description = "Get the current search backend configuration and status.")]
    async fn get_config(&self) -> Result<CallToolResult, McpError> {
        #[derive(Serialize)]
        struct ConfigStatus {
            backend: String,
            available: bool,
            max_results: usize,
            cache_enabled: bool,
        }

        let status = ConfigStatus {
            backend: self.backend.name().to_string(),
            available: self.backend.is_available(),
            max_results: self.config.search.max_results,
            cache_enabled: self.config.search.cache_enabled,
        };

        json_success(&status)
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for WebSearchMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Web Search MCP Server - provides tools for searching the web using \
                 SearXNG (self-hosted meta-search engine). Supports web search, \
                 news search, and image search. No API keys required."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for WebSearchMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
