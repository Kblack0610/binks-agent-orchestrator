//! HTTP fetch service for retrieving and parsing web content
//!
//! Provides URL fetching, JSON parsing, CSS selector extraction,
//! and HTML-to-markdown conversion.

pub mod types;

use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use crate::config::FetchConfig;
use types::{FetchResult, ParseHtmlResult, ParsedElement};

/// HTTP fetch service with configurable client
#[derive(Clone)]
pub struct FetchService {
    client: Client,
    config: FetchConfig,
}

impl FetchService {
    /// Create a new FetchService with the given configuration
    pub fn new(config: &FetchConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            config: config.clone(),
        }
    }

    /// Fetch a URL and return the response as text
    pub async fn fetch(&self, url: &str) -> Result<FetchResult> {
        let response = self.client.get(url).send().await?;
        let status_code = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Check Content-Length before downloading
        if let Some(len) = response.content_length() {
            if len as usize > self.config.max_response_size {
                return Err(anyhow!(
                    "Response too large: {} bytes (max: {} bytes)",
                    len,
                    self.config.max_response_size
                ));
            }
        }

        let content = response.text().await?;
        let content_length = content.len();

        if content_length > self.config.max_response_size {
            return Err(anyhow!(
                "Response too large: {} bytes (max: {} bytes)",
                content_length,
                self.config.max_response_size
            ));
        }

        Ok(FetchResult {
            url: url.to_string(),
            status_code,
            content_type,
            content,
            content_length,
        })
    }

    /// Fetch a URL and parse the response as JSON
    pub async fn fetch_json(&self, url: &str) -> Result<serde_json::Value> {
        let result = self.fetch(url).await?;

        if result.status_code >= 400 {
            return Err(anyhow!(
                "HTTP error {}: {}",
                result.status_code,
                &result.content[..result.content.len().min(200)]
            ));
        }

        let value: serde_json::Value = serde_json::from_str(&result.content)?;
        Ok(value)
    }

    /// Fetch a URL and extract elements matching a CSS selector
    pub async fn parse_html(&self, url: &str, selector: &str) -> Result<ParseHtmlResult> {
        let result = self.fetch(url).await?;

        if result.status_code >= 400 {
            return Err(anyhow!(
                "HTTP error {}: {}",
                result.status_code,
                &result.content[..result.content.len().min(200)]
            ));
        }

        let document = scraper::Html::parse_document(&result.content);
        let css_selector = scraper::Selector::parse(selector)
            .map_err(|e| anyhow!("Invalid CSS selector '{}': {:?}", selector, e))?;

        let elements: Vec<ParsedElement> = document
            .select(&css_selector)
            .map(|el| {
                let attributes: HashMap<String, String> = el
                    .value()
                    .attrs()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();

                ParsedElement {
                    text: el.text().collect::<Vec<_>>().join(""),
                    html: el.inner_html(),
                    tag: el.value().name().to_string(),
                    attributes,
                }
            })
            .collect();

        Ok(ParseHtmlResult {
            url: url.to_string(),
            selector: selector.to_string(),
            count: elements.len(),
            elements,
        })
    }

    /// Fetch a URL and convert the HTML to markdown
    pub async fn fetch_markdown(&self, url: &str) -> Result<String> {
        let result = self.fetch(url).await?;

        if result.status_code >= 400 {
            return Err(anyhow!(
                "HTTP error {}: {}",
                result.status_code,
                &result.content[..result.content.len().min(200)]
            ));
        }

        // Strip <style> and <script> tags before converting to markdown
        // to avoid CSS/JS content leaking into the output
        static STYLE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap());
        static SCRIPT_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap());
        let cleaned = STYLE_RE.replace_all(&result.content, "");
        let cleaned = SCRIPT_RE.replace_all(&cleaned, "");

        Ok(html2md::parse_html(&cleaned))
    }
}
