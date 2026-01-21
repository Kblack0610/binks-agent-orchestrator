//! Configuration loading for web-search-mcp
//!
//! Configuration is loaded from:
//! 1. Environment variable SEARXNG_URL
//! 2. Environment variable WEB_SEARCH_CONFIG_PATH
//! 3. ~/.binks/web-search.toml
//! 4. Default values

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Search configuration
    #[serde(default)]
    pub search: SearchConfig,
    /// SearXNG specific configuration
    #[serde(default)]
    pub searxng: SearXNGConfig,
}

/// General search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Maximum number of results to return
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    /// Enable result caching
    #[serde(default = "default_true")]
    pub cache_enabled: bool,
    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,
}

/// SearXNG configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearXNGConfig {
    /// SearXNG instance URL
    #[serde(default = "default_searxng_url")]
    pub url: String,
    /// Engines to use (comma-separated, empty = use instance defaults)
    #[serde(default)]
    pub engines: String,
}

// Default value functions
fn default_max_results() -> usize {
    10
}

fn default_true() -> bool {
    true
}

fn default_cache_ttl() -> u64 {
    300 // 5 minutes
}

fn default_searxng_url() -> String {
    "http://localhost:8080".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            search: SearchConfig::default(),
            searxng: SearXNGConfig::default(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: default_max_results(),
            cache_enabled: default_true(),
            cache_ttl_seconds: default_cache_ttl(),
        }
    }
}

impl Default for SearXNGConfig {
    fn default() -> Self {
        Self {
            url: default_searxng_url(),
            engines: String::new(),
        }
    }
}

impl Config {
    /// Load configuration from file or use defaults
    pub fn load() -> Result<Self> {
        let config_path = Self::find_config_path();

        let mut config = if let Some(path) = config_path {
            if path.exists() {
                tracing::info!("Loading config from: {}", path.display());
                let content = std::fs::read_to_string(&path)?;
                toml::from_str(&content)?
            } else {
                tracing::info!("Config file not found, using defaults");
                Self::default()
            }
        } else {
            tracing::info!("No config path specified, using defaults");
            Self::default()
        };

        // SearXNG URL from environment (highest priority)
        if let Ok(url) = std::env::var("SEARXNG_URL") {
            config.searxng.url = url;
        }

        Ok(config)
    }

    /// Find the configuration file path
    fn find_config_path() -> Option<PathBuf> {
        // 1. Check environment variable
        if let Ok(path) = std::env::var("WEB_SEARCH_CONFIG_PATH") {
            return Some(PathBuf::from(path));
        }

        // 2. Check ~/.binks/web-search.toml
        if let Ok(home) = std::env::var("HOME") {
            let path = PathBuf::from(home).join(".binks").join("web-search.toml");
            return Some(path);
        }

        None
    }
}
