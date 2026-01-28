//! Types for HTTP fetch results

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of fetching a URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    /// The fetched URL
    pub url: String,
    /// HTTP status code
    pub status_code: u16,
    /// Content-Type header value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Response body as text
    pub content: String,
    /// Content length in bytes
    pub content_length: usize,
}

/// A single element extracted via CSS selector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedElement {
    /// Text content of the element
    pub text: String,
    /// Inner HTML of the element
    pub html: String,
    /// Tag name (e.g. "div", "a", "p")
    pub tag: String,
    /// Element attributes
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, String>,
}

/// Result of parsing HTML with a CSS selector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseHtmlResult {
    /// The source URL
    pub url: String,
    /// The CSS selector used
    pub selector: String,
    /// Number of matching elements
    pub count: usize,
    /// The matched elements
    pub elements: Vec<ParsedElement>,
}
