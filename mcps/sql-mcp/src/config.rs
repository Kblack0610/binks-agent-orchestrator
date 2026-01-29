//! Configuration for SQL MCP Server

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

/// SQL MCP configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SqlConfig {
    /// Database connection settings
    pub database: DatabaseConfig,
}

/// Database connection configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file
    pub path: PathBuf,

    /// Whether to allow write operations (INSERT, UPDATE, DELETE, etc.)
    /// Default: false (read-only)
    #[serde(default)]
    pub allow_writes: bool,

    /// Maximum query execution time in seconds
    /// Default: 30
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    30
}

impl SqlConfig {
    /// Load configuration from file
    ///
    /// Looks for config in:
    /// 1. `SQL_CONFIG_PATH` environment variable
    /// 2. `~/.binks/sql.toml`
    pub fn load() -> Result<Self> {
        let config_path = if let Ok(path) = std::env::var("SQL_CONFIG_PATH") {
            PathBuf::from(path)
        } else {
            dirs::home_dir()
                .context("Could not determine home directory")?
                .join(".binks")
                .join("sql.toml")
        };

        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config from {:?}", config_path))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {:?}", config_path))
    }

    /// Create a default config pointing to a specific database
    #[allow(dead_code)]
    pub fn with_database(path: PathBuf) -> Self {
        Self {
            database: DatabaseConfig {
                path,
                allow_writes: false,
                timeout_secs: default_timeout(),
            },
        }
    }
}

impl Default for SqlConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                path: PathBuf::from("database.db"),
                allow_writes: false,
                timeout_secs: default_timeout(),
            },
        }
    }
}
