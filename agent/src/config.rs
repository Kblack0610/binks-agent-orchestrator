//! Configuration loading

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Find a config file by walking up the directory tree, then checking global config.
///
/// Search order:
/// 1. Current directory and parent directories (walking up to root)
/// 2. Global config at ~/.config/binks/
///
/// Returns the path if found, None otherwise.
fn find_config_file(filename: &str) -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    // Walk up the directory tree
    loop {
        // Check current directory
        let candidate = current.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }

        // Also check agent/ subdirectory (for project root detection)
        let agent_candidate = current.join("agent").join(filename);
        if agent_candidate.exists() {
            return Some(agent_candidate);
        }

        // Move to parent directory
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break, // Reached filesystem root
        }
    }

    // Fallback: Check global config
    if let Some(config_dir) = dirs::config_dir() {
        let global_path = config_dir.join("binks").join(filename);
        if global_path.exists() {
            return Some(global_path);
        }
    }

    None
}

/// MCP server configuration (from .mcp.json)
#[derive(Debug, Clone, Deserialize)]
pub struct McpConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl McpConfig {
    /// Load MCP config from .mcp.json
    ///
    /// Search order:
    /// 1. Walk up directory tree from cwd looking for .mcp.json
    /// 2. Check ~/.config/binks/.mcp.json (global fallback)
    pub fn load() -> Result<Option<Self>> {
        if let Some(config_path) = find_config_file(".mcp.json") {
            tracing::debug!("Loading MCP config from: {}", config_path.display());
            return Self::load_from_path(&config_path).map(Some);
        }

        tracing::debug!("No .mcp.json found");
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
    "qwen3-coder:30b".to_string()
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
    /// Load config from .agent.toml
    ///
    /// Search order:
    /// 1. Walk up directory tree from cwd looking for .agent.toml
    /// 2. Check ~/.config/binks/.agent.toml (global fallback)
    /// 3. Fall back to defaults
    pub fn load() -> Result<Self> {
        if let Some(config_path) = find_config_file(".agent.toml") {
            tracing::debug!("Loading config from: {}", config_path.display());
            return Self::load_from_path(&config_path);
        }

        // No config file found, return defaults
        tracing::debug!("No .agent.toml found, using defaults");
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
