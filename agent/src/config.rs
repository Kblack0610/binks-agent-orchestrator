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
    /// Load MCP config from .mcp.json in the current directory only
    pub fn load() -> Result<Option<Self>> {
        let config_path = std::env::current_dir()?.join(".mcp.json");

        if config_path.exists() {
            return Self::load_from_path(&config_path).map(Some);
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

// ============================================================================
// Agent Configuration (.agent.toml)
// ============================================================================

/// Top-level agent configuration (from .agent.toml)
#[derive(Debug, Default, Deserialize)]
pub struct AgentFileConfig {
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub agent: AgentSectionConfig,
    #[serde(default)]
    pub monitor: MonitorSectionConfig,
}

/// LLM configuration section
#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_ollama_url")]
    pub url: String,
    #[serde(default = "default_model")]
    pub model: String,
}

/// Agent configuration section
#[derive(Debug, Default, Deserialize)]
pub struct AgentSectionConfig {
    pub system_prompt: Option<String>,
}

/// Monitor configuration section
#[derive(Debug, Deserialize)]
pub struct MonitorSectionConfig {
    #[serde(default = "default_interval")]
    pub interval: u64,
    #[serde(default)]
    pub repos: Vec<String>,
}

// Default value functions
fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_model() -> String {
    "qwen2.5-coder:32b".to_string()
}

fn default_interval() -> u64 {
    300
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            url: default_ollama_url(),
            model: default_model(),
        }
    }
}

impl Default for MonitorSectionConfig {
    fn default() -> Self {
        Self {
            interval: default_interval(),
            repos: Vec::new(),
        }
    }
}

impl AgentFileConfig {
    /// Load config from .agent.toml in the current directory only
    pub fn load() -> Result<Self> {
        let config_path = std::env::current_dir()?.join(".agent.toml");

        if config_path.exists() {
            return Self::load_from_path(&config_path);
        }

        // No config file found, return defaults
        Ok(Self::default())
    }

    /// Load from a specific path
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: AgentFileConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get the default model (for use elsewhere)
    pub fn default_model() -> String {
        default_model()
    }

    /// Get the default Ollama URL (for use elsewhere)
    pub fn default_ollama_url() -> String {
        default_ollama_url()
    }
}
