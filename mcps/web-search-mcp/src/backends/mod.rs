//! Search backend implementations
//!
//! This module provides a trait-based abstraction for search backends.
//! Currently supports SearXNG (self-hosted meta-search engine).

use anyhow::Result;
use async_trait::async_trait;

use crate::types::{ImageResults, NewsResults, SearchResults};

pub mod searxng;

/// Trait for search backends
///
/// All search backends must implement this trait to provide a consistent
/// interface for the MCP server.
#[async_trait]
pub trait SearchBackend: Send + Sync {
    /// Get the name of this backend
    fn name(&self) -> &str;

    /// Perform a web search
    async fn search(&self, query: &str, limit: usize) -> Result<SearchResults>;

    /// Perform a news search
    async fn search_news(&self, query: &str, limit: usize) -> Result<NewsResults>;

    /// Perform an image search
    async fn search_images(&self, query: &str, limit: usize) -> Result<ImageResults>;

    /// Check if this backend is configured and available
    fn is_available(&self) -> bool;
}
