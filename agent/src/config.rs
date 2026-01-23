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
    /// MCP tier level (1=essential, 2=standard, 3=extended, 4=agent-only)
    /// Used for automatic filtering based on model size
    #[serde(default = "default_tier")]
    pub tier: u8,
}

fn default_tier() -> u8 {
    2 // Standard tier by default for backwards compatibility
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
    #[serde(default)]
    pub mcp: McpSectionConfig,
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

// ============================================================================
// MCP Filtering Configuration
// ============================================================================

/// MCP filtering configuration section
#[derive(Debug, Deserialize)]
pub struct McpSectionConfig {
    /// Enable automatic model-based filtering (default: true)
    #[serde(default = "default_auto_filter")]
    pub auto_filter: bool,

    /// Size classification thresholds (in billions of parameters)
    #[serde(default)]
    pub size_thresholds: SizeThresholds,

    /// Per-size-class profiles
    #[serde(default)]
    pub profiles: McpProfiles,
}

fn default_auto_filter() -> bool {
    true
}

impl Default for McpSectionConfig {
    fn default() -> Self {
        Self {
            auto_filter: default_auto_filter(),
            size_thresholds: SizeThresholds::default(),
            profiles: McpProfiles::default(),
        }
    }
}

/// Size classification thresholds
#[derive(Debug, Deserialize)]
pub struct SizeThresholds {
    /// Upper bound for "small" models (inclusive, in billions)
    #[serde(default = "default_small_threshold")]
    pub small: u32,
    /// Upper bound for "medium" models (inclusive, in billions)
    #[serde(default = "default_medium_threshold")]
    pub medium: u32,
}

fn default_small_threshold() -> u32 {
    8
}
fn default_medium_threshold() -> u32 {
    32
}

impl Default for SizeThresholds {
    fn default() -> Self {
        Self {
            small: default_small_threshold(),
            medium: default_medium_threshold(),
        }
    }
}

/// Per-size-class MCP profiles
#[derive(Debug, Deserialize)]
pub struct McpProfiles {
    #[serde(default = "default_small_profile")]
    pub small: McpProfile,
    #[serde(default = "default_medium_profile")]
    pub medium: McpProfile,
    #[serde(default = "default_large_profile")]
    pub large: McpProfile,
}

fn default_small_profile() -> McpProfile {
    McpProfile { max_tier: 1, servers: None }
}

fn default_medium_profile() -> McpProfile {
    McpProfile { max_tier: 2, servers: None }
}

fn default_large_profile() -> McpProfile {
    McpProfile { max_tier: 3, servers: None }
}

impl Default for McpProfiles {
    fn default() -> Self {
        Self {
            small: default_small_profile(),
            medium: default_medium_profile(),
            large: default_large_profile(),
        }
    }
}

/// Configuration for a specific model size class
#[derive(Debug, Deserialize)]
pub struct McpProfile {
    /// Maximum tier level for this size class
    #[serde(default = "default_max_tier")]
    pub max_tier: u8,
    /// Optional explicit server list (overrides tier filtering)
    pub servers: Option<Vec<String>>,
}

fn default_max_tier() -> u8 {
    2
}

impl Default for McpProfile {
    fn default() -> Self {
        Self {
            max_tier: default_max_tier(),
            servers: None,
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
