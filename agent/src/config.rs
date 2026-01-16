//! Configuration loading

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// MCP server configuration (from .mcp.json)
#[derive(Debug, Deserialize)]
pub struct McpConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl McpConfig {
    /// Load MCP config from .mcp.json, searching up the directory tree
    pub fn load() -> Result<Option<Self>> {
        let mut dir = std::env::current_dir()?;

        loop {
            let config_path = dir.join(".mcp.json");
            if config_path.exists() {
                return Self::load_from_path(&config_path).map(Some);
            }

            if !dir.pop() {
                break;
            }
        }

        Ok(None)
    }

    /// Load from a specific path
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: McpConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
}
