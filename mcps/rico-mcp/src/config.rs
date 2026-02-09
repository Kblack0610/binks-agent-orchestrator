//! Configuration for RICO dataset paths and thresholds

use std::path::PathBuf;
use thiserror::Error;

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Vectors file not found: {0}. Run scripts/download-rico.sh first.")]
    VectorsNotFound(String),
    #[error("Metadata file not found: {0}. Run scripts/download-rico.sh first.")]
    MetadataNotFound(String),
}

/// Configuration for the RICO MCP server
#[derive(Clone, Debug)]
pub struct RicoConfig {
    /// Base directory for RICO data files
    pub data_dir: PathBuf,
    /// Directory for cached screenshots
    pub screenshot_cache_dir: PathBuf,
    /// Maximum number of screens to cache in LRU
    pub cache_size: usize,
    /// Default number of results for similarity search
    pub default_top_k: usize,
    /// Minimum similarity threshold (0.0-1.0)
    pub min_similarity: f32,
}

impl Default for RicoConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let data_dir = home.join(".rico-mcp").join("data");
        let screenshot_cache_dir = home.join(".rico-mcp").join("screenshots");

        Self {
            data_dir,
            screenshot_cache_dir,
            cache_size: 1000,
            default_top_k: 10,
            min_similarity: 0.5,
        }
    }
}

impl RicoConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(dir) = std::env::var("RICO_DATA_DIR") {
            config.data_dir = PathBuf::from(dir);
        }

        if let Ok(dir) = std::env::var("RICO_SCREENSHOT_DIR") {
            config.screenshot_cache_dir = PathBuf::from(dir);
        }

        if let Ok(size) = std::env::var("RICO_CACHE_SIZE") {
            if let Ok(s) = size.parse() {
                config.cache_size = s;
            }
        }

        if let Ok(k) = std::env::var("RICO_DEFAULT_TOP_K") {
            if let Ok(k) = k.parse() {
                config.default_top_k = k;
            }
        }

        config
    }

    /// Path to the UI layout vectors NPY file
    pub fn vectors_path(&self) -> PathBuf {
        self.data_dir.join("ui_layout_vectors.npy")
    }

    /// Path to the UI metadata JSON file
    pub fn metadata_path(&self) -> PathBuf {
        self.data_dir.join("ui_metadata.json")
    }

    /// Path to semantic annotations directory
    pub fn annotations_dir(&self) -> PathBuf {
        self.data_dir.join("semantic_annotations")
    }

    /// Path to screenshot for a given screen ID
    pub fn screenshot_path(&self, screen_id: u32) -> PathBuf {
        self.screenshot_cache_dir.join(format!("{}.jpg", screen_id))
    }

    /// Check if required data files exist
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !self.vectors_path().exists() {
            return Err(ConfigError::VectorsNotFound(
                self.vectors_path().display().to_string(),
            ));
        }
        if !self.metadata_path().exists() {
            return Err(ConfigError::MetadataNotFound(
                self.metadata_path().display().to_string(),
            ));
        }
        Ok(())
    }
}
