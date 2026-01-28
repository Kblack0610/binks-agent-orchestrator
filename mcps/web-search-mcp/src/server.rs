//! MCP Server implementation for web search
//!
//! This module defines the main MCP server that exposes web search
//! tools with pluggable backend support.

use mcp_common::{json_success, text_success, CallToolResult, McpError, ResultExt};
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
use crate::fetch::FetchService;

/// The main Web Search MCP Server
#[derive(Clone)]
pub struct WebSearchMcpServer {
    backend: Arc<dyn SearchBackend>,
    fetch_service: FetchService,
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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FetchParams {
    /// The URL to fetch
    #[schemars(description = "The URL to fetch content from")]
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FetchJsonParams {
    /// The URL to fetch JSON from
    #[schemars(description = "The URL to fetch and parse as JSON")]
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ParseHtmlParams {
    /// The URL to fetch HTML from
    #[schemars(description = "The URL to fetch HTML from")]
    pub url: String,
    /// CSS selector to extract elements
    #[schemars(description = "CSS selector to extract matching elements (e.g., 'h1', '.class', '#id', 'div > p')")]
    pub selector: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FetchMarkdownParams {
    /// The URL to fetch and convert to markdown
    #[schemars(description = "The URL to fetch and convert HTML to markdown")]
    pub url: String,
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

        let fetch_service = FetchService::new(&config.fetch);

        Self {
            backend,
            fetch_service,
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

    // ========================================================================
    // Fetch Tools
    // ========================================================================

    #[tool(description = "Fetch a URL and return the raw response content with status code, content type, and body.")]
    async fn fetch(
        &self,
        Parameters(params): Parameters<FetchParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("Fetching URL: {}", params.url);

        let result = self
            .fetch_service
            .fetch(&params.url)
            .await
            .to_mcp_err()?;

        json_success(&result)
    }

    #[tool(description = "Fetch a URL and parse the response as JSON. Returns the parsed JSON value.")]
    async fn fetch_json(
        &self,
        Parameters(params): Parameters<FetchJsonParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("Fetching JSON from: {}", params.url);

        let value = self
            .fetch_service
            .fetch_json(&params.url)
            .await
            .to_mcp_err()?;

        json_success(&value)
    }

    #[tool(description = "Fetch a URL and extract HTML elements matching a CSS selector. Returns matching elements with text, HTML, tag names, and attributes.")]
    async fn parse_html(
        &self,
        Parameters(params): Parameters<ParseHtmlParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(
            "Parsing HTML from {} with selector: {}",
            params.url,
            params.selector
        );

        let result = self
            .fetch_service
            .parse_html(&params.url, &params.selector)
            .await
            .to_mcp_err()?;

        json_success(&result)
    }

    #[tool(description = "Fetch a URL and convert the HTML content to markdown format.")]
    async fn fetch_markdown(
        &self,
        Parameters(params): Parameters<FetchMarkdownParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("Fetching markdown from: {}", params.url);

        let markdown = self
            .fetch_service
            .fetch_markdown(&params.url)
            .await
            .to_mcp_err()?;

        Ok(text_success(markdown))
    }

    // ========================================================================
    // Configuration
    // ========================================================================

    #[tool(description = "Get the current search backend configuration and status.")]
    async fn get_config(&self) -> Result<CallToolResult, McpError> {
        #[derive(Serialize)]
        struct ConfigStatus {
            backend: String,
            available: bool,
            max_results: usize,
            cache_enabled: bool,
            fetch_timeout_seconds: u64,
            fetch_max_response_size: usize,
        }

        let status = ConfigStatus {
            backend: self.backend.name().to_string(),
            available: self.backend.is_available(),
            max_results: self.config.search.max_results,
            cache_enabled: self.config.search.cache_enabled,
            fetch_timeout_seconds: self.config.fetch.timeout_seconds,
            fetch_max_response_size: self.config.fetch.max_response_size,
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
                 news search, and image search. No API keys required. \
                 Also provides HTTP fetch tools for retrieving web content, \
                 parsing JSON, extracting HTML elements via CSS selectors, \
                 and converting HTML to markdown."
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
