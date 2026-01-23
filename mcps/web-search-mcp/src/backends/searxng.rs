//! SearXNG backend
//!
//! Implements the SearchBackend trait using a self-hosted SearXNG instance.
//! See: https://docs.searxng.org/dev/search_api.html

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use super::SearchBackend;
use crate::config::SearXNGConfig;
use crate::types::{
    ImageResult, ImageResults, NewsResult, NewsResults, SearchResult, SearchResults,
};

/// SearXNG backend
pub struct SearXNGBackend {
    client: Client,
    config: SearXNGConfig,
}

impl SearXNGBackend {
    pub fn new(config: SearXNGConfig) -> Self {
        let client = Client::builder()
            .user_agent("web-search-mcp/0.1")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }
}

// SearXNG API response types
#[derive(Debug, Deserialize)]
struct SearXNGResponse {
    results: Vec<SearXNGResult>,
    number_of_results: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SearXNGResult {
    title: String,
    url: String,
    content: Option<String>,
    engine: Option<String>,
    #[serde(rename = "publishedDate")]
    published_date: Option<String>,
    img_src: Option<String>,
    thumbnail_src: Option<String>,
    img_format: Option<String>,
}

#[async_trait]
impl SearchBackend for SearXNGBackend {
    fn name(&self) -> &str {
        "searxng"
    }

    fn is_available(&self) -> bool {
        !self.config.url.is_empty()
    }

    async fn search(&self, query: &str, limit: usize) -> Result<SearchResults> {
        if !self.is_available() {
            return Err(anyhow!("SearXNG URL not configured"));
        }

        let url = format!("{}/search", self.config.url);

        let mut params = vec![
            ("q", query.to_string()),
            ("format", "json".to_string()),
            ("pageno", "1".to_string()),
        ];

        if !self.config.engines.is_empty() {
            params.push(("engines", self.config.engines.clone()));
        }

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("SearXNG error {}: {}", status, text));
        }

        let searxng_response: SearXNGResponse = response.json().await?;

        let results: Vec<SearchResult> = searxng_response
            .results
            .into_iter()
            // Filter out image-only results (those with actual image URLs, not empty strings)
            .filter(|r| r.img_src.as_ref().map_or(true, |s| s.is_empty()))
            .take(limit)
            .map(|r| SearchResult {
                title: r.title,
                url: r.url,
                description: r.content.unwrap_or_default(),
                source: r.engine,
                published: r.published_date,
            })
            .collect();

        Ok(SearchResults {
            query: query.to_string(),
            total: searxng_response.number_of_results,
            results,
            backend: self.name().to_string(),
        })
    }

    async fn search_news(&self, query: &str, limit: usize) -> Result<NewsResults> {
        if !self.is_available() {
            return Err(anyhow!("SearXNG URL not configured"));
        }

        let url = format!("{}/search", self.config.url);

        let response = self
            .client
            .get(&url)
            .query(&[
                ("q", query),
                ("format", "json"),
                ("categories", "news"),
                ("pageno", "1"),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("SearXNG error {}: {}", status, text));
        }

        let searxng_response: SearXNGResponse = response.json().await?;

        let results: Vec<NewsResult> = searxng_response
            .results
            .into_iter()
            .take(limit)
            .map(|r| NewsResult {
                title: r.title,
                url: r.url,
                description: r.content.unwrap_or_default(),
                source: r.engine.unwrap_or_else(|| "Unknown".to_string()),
                published: r.published_date,
                thumbnail: r.thumbnail_src,
            })
            .collect();

        Ok(NewsResults {
            query: query.to_string(),
            results,
            backend: self.name().to_string(),
        })
    }

    async fn search_images(&self, query: &str, limit: usize) -> Result<ImageResults> {
        if !self.is_available() {
            return Err(anyhow!("SearXNG URL not configured"));
        }

        let url = format!("{}/search", self.config.url);

        let response = self
            .client
            .get(&url)
            .query(&[
                ("q", query),
                ("format", "json"),
                ("categories", "images"),
                ("pageno", "1"),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("SearXNG error {}: {}", status, text));
        }

        let searxng_response: SearXNGResponse = response.json().await?;

        let results: Vec<ImageResult> = searxng_response
            .results
            .into_iter()
            .take(limit)
            .filter_map(|r| {
                r.img_src.map(|img_url| {
                    // Parse dimensions from img_format if available (e.g., "1920x1080")
                    let (width, height) = r
                        .img_format
                        .as_ref()
                        .and_then(|f| {
                            let parts: Vec<&str> = f.split('x').collect();
                            if parts.len() == 2 {
                                Some((
                                    parts[0].parse().ok(),
                                    parts[1].parse().ok(),
                                ))
                            } else {
                                None
                            }
                        })
                        .unwrap_or((None, None));

                    ImageResult {
                        title: r.title.clone(),
                        image_url: img_url,
                        page_url: r.url.clone(),
                        width,
                        height,
                        source: r.engine.clone(),
                    }
                })
            })
            .collect();

        Ok(ImageResults {
            query: query.to_string(),
            results,
            backend: self.name().to_string(),
        })
    }
}
