//! Common types for web search results
//!
//! These types are used across all search backends to provide a consistent
//! interface for search results.

use serde::{Deserialize, Serialize};

/// A single web search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The title of the result
    pub title: String,
    /// The URL of the result
    pub url: String,
    /// A description or snippet of the result
    pub description: String,
    /// The source/domain of the result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// When the content was published (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
}

/// A collection of search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// The search query that was executed
    pub query: String,
    /// Total number of results found (may be estimated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// The search results
    pub results: Vec<SearchResult>,
    /// The backend that was used
    pub backend: String,
}

/// A news search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsResult {
    /// The title of the article
    pub title: String,
    /// The URL of the article
    pub url: String,
    /// A description or snippet of the article
    pub description: String,
    /// The news source
    pub source: String,
    /// When the article was published
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
    /// Thumbnail image URL (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

/// A collection of news search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsResults {
    /// The search query that was executed
    pub query: String,
    /// The news results
    pub results: Vec<NewsResult>,
    /// The backend that was used
    pub backend: String,
}

/// An image search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResult {
    /// The title/alt text of the image
    pub title: String,
    /// The URL of the image
    pub image_url: String,
    /// The URL of the page containing the image
    pub page_url: String,
    /// Image width (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Image height (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// The source/domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// A collection of image search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResults {
    /// The search query that was executed
    pub query: String,
    /// The image results
    pub results: Vec<ImageResult>,
    /// The backend that was used
    pub backend: String,
}
