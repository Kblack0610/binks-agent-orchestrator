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
#[derive(Debug, Deserialize)]
pub struct AgentSectionConfig {
    pub system_prompt: Option<String>,
    /// Maximum number of tool-calling iterations (default: 10)
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// LLM request timeout in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_llm_timeout_secs")]
    pub llm_timeout_secs: u64,
    /// Tool execution timeout in seconds (default: 60 = 1 minute)
    #[serde(default = "default_tool_timeout_secs")]
    pub tool_timeout_secs: u64,
    /// Maximum conversation history messages to keep (default: 100)
    #[serde(default = "default_max_history_messages")]
    pub max_history_messages: usize,
}

fn default_max_iterations() -> usize {
    10
}

fn default_llm_timeout_secs() -> u64 {
    300 // 5 minutes
}

fn default_tool_timeout_secs() -> u64 {
    60 // 1 minute
}

fn default_max_history_messages() -> usize {
    100
}

impl Default for AgentSectionConfig {
    fn default() -> Self {
        Self {
            system_prompt: None,
            max_iterations: default_max_iterations(),
            llm_timeout_secs: default_llm_timeout_secs(),
            tool_timeout_secs: default_tool_timeout_secs(),
            max_history_messages: default_max_history_messages(),
        }
    }
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
    McpProfile {
        max_tier: 1,
        servers: None,
    }
}

fn default_medium_profile() -> McpProfile {
    McpProfile {
        max_tier: 2,
        servers: None,
    }
}

fn default_large_profile() -> McpProfile {
    McpProfile {
        max_tier: 3,
        servers: None,
    }
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ============== MCP Config Tests ==============

    #[test]
    fn test_mcp_config_parse_minimal() {
        let json = r#"{
            "mcpServers": {
                "test": {
                    "command": "/bin/test"
                }
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let config = McpConfig::load_from_path(file.path()).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);
        assert_eq!(config.mcp_servers["test"].command, "/bin/test");
        assert!(config.mcp_servers["test"].args.is_empty());
        assert!(config.mcp_servers["test"].env.is_empty());
        // Default tier is 2
        assert_eq!(config.mcp_servers["test"].tier, 2);
    }

    #[test]
    fn test_mcp_config_parse_full() {
        let json = r#"{
            "mcpServers": {
                "sysinfo": {
                    "command": "./mcps/sysinfo-mcp/target/release/sysinfo-mcp",
                    "args": ["--verbose"],
                    "env": {"LOG_LEVEL": "debug"},
                    "tier": 1
                },
                "github": {
                    "command": "gh",
                    "args": ["mcp", "serve"],
                    "tier": 3
                }
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let config = McpConfig::load_from_path(file.path()).unwrap();
        assert_eq!(config.mcp_servers.len(), 2);

        let sysinfo = &config.mcp_servers["sysinfo"];
        assert_eq!(sysinfo.tier, 1);
        assert_eq!(sysinfo.args, vec!["--verbose"]);
        assert_eq!(sysinfo.env.get("LOG_LEVEL").unwrap(), "debug");

        let github = &config.mcp_servers["github"];
        assert_eq!(github.tier, 3);
        assert_eq!(github.args, vec!["mcp", "serve"]);
    }

    #[test]
    fn test_mcp_config_invalid_json() {
        let json = r#"{ invalid json "#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let result = McpConfig::load_from_path(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_mcp_config_missing_required_field() {
        // Missing "command" field
        let json = r#"{
            "mcpServers": {
                "test": {
                    "args": []
                }
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let result = McpConfig::load_from_path(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_mcp_config_empty_servers() {
        let json = r#"{"mcpServers": {}}"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let config = McpConfig::load_from_path(file.path()).unwrap();
        assert!(config.mcp_servers.is_empty());
    }

    // ============== Agent Config Tests ==============

    #[test]
    fn test_agent_config_defaults() {
        let config = AgentFileConfig::default();

        assert_eq!(config.llm.url, "http://localhost:11434");
        assert_eq!(config.llm.model, "qwen3-coder:30b");
        assert_eq!(config.monitor.interval, 300);
        assert!(config.monitor.repos.is_empty());
        assert!(config.agent.system_prompt.is_none());
        assert!(config.mcp.auto_filter);
    }

    #[test]
    fn test_agent_config_parse_minimal() {
        let toml = r#"
[llm]
url = "http://192.168.1.4:11434"
model = "llama3.1:8b"
"#;

        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        file.write_all(toml.as_bytes()).unwrap();

        let config = AgentFileConfig::load_from_path(file.path()).unwrap();
        assert_eq!(config.llm.url, "http://192.168.1.4:11434");
        assert_eq!(config.llm.model, "llama3.1:8b");
        // Defaults should still apply for unspecified sections
        assert_eq!(config.monitor.interval, 300);
    }

    #[test]
    fn test_agent_config_parse_full() {
        let toml = r#"
[llm]
url = "http://gpu-server:11434"
model = "codestral:22b"

[agent]
system_prompt = "You are a helpful coding assistant."

[monitor]
interval = 60
repos = ["owner/repo1", "owner/repo2"]

[mcp]
auto_filter = false

[mcp.size_thresholds]
small = 7
medium = 30

[mcp.profiles.small]
max_tier = 1
servers = ["sysinfo"]

[mcp.profiles.medium]
max_tier = 2

[mcp.profiles.large]
max_tier = 4
"#;

        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        file.write_all(toml.as_bytes()).unwrap();

        let config = AgentFileConfig::load_from_path(file.path()).unwrap();

        // LLM section
        assert_eq!(config.llm.url, "http://gpu-server:11434");
        assert_eq!(config.llm.model, "codestral:22b");

        // Agent section
        assert_eq!(
            config.agent.system_prompt.as_deref(),
            Some("You are a helpful coding assistant.")
        );

        // Monitor section
        assert_eq!(config.monitor.interval, 60);
        assert_eq!(config.monitor.repos, vec!["owner/repo1", "owner/repo2"]);

        // MCP section
        assert!(!config.mcp.auto_filter);
        assert_eq!(config.mcp.size_thresholds.small, 7);
        assert_eq!(config.mcp.size_thresholds.medium, 30);
        assert_eq!(config.mcp.profiles.small.max_tier, 1);
        assert_eq!(
            config.mcp.profiles.small.servers.as_ref().unwrap(),
            &vec!["sysinfo".to_string()]
        );
        assert_eq!(config.mcp.profiles.large.max_tier, 4);
    }

    #[test]
    fn test_agent_config_invalid_toml() {
        let toml = r#"[invalid toml syntax"#;

        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        file.write_all(toml.as_bytes()).unwrap();

        let result = AgentFileConfig::load_from_path(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_config_empty_file() {
        let toml = "";

        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        file.write_all(toml.as_bytes()).unwrap();

        // Empty file should use all defaults
        let config = AgentFileConfig::load_from_path(file.path()).unwrap();
        assert_eq!(config.llm.url, "http://localhost:11434");
    }

    #[test]
    fn test_agent_config_partial_sections() {
        let toml = r#"
[llm]
model = "custom-model"
# url not specified, should use default
"#;

        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        file.write_all(toml.as_bytes()).unwrap();

        let config = AgentFileConfig::load_from_path(file.path()).unwrap();
        assert_eq!(config.llm.model, "custom-model");
        assert_eq!(config.llm.url, "http://localhost:11434"); // default
    }

    // ============== Size Threshold Tests ==============

    #[test]
    fn test_size_thresholds_defaults() {
        let thresholds = SizeThresholds::default();
        assert_eq!(thresholds.small, 8);
        assert_eq!(thresholds.medium, 32);
    }

    // ============== MCP Profile Tests ==============

    #[test]
    fn test_mcp_profiles_defaults() {
        let profiles = McpProfiles::default();

        assert_eq!(profiles.small.max_tier, 1);
        assert!(profiles.small.servers.is_none());

        assert_eq!(profiles.medium.max_tier, 2);
        assert!(profiles.medium.servers.is_none());

        assert_eq!(profiles.large.max_tier, 3);
        assert!(profiles.large.servers.is_none());
    }

    #[test]
    fn test_mcp_section_defaults() {
        let mcp = McpSectionConfig::default();
        assert!(mcp.auto_filter);
        assert_eq!(mcp.size_thresholds.small, 8);
        assert_eq!(mcp.size_thresholds.medium, 32);
    }

    // ============== Tier Default Tests ==============

    #[test]
    fn test_tier_defaults_to_two() {
        assert_eq!(default_tier(), 2);
    }

    #[test]
    fn test_max_tier_defaults_to_two() {
        assert_eq!(default_max_tier(), 2);
    }

    // ============== Static Method Tests ==============

    #[test]
    fn test_default_model_value() {
        assert_eq!(AgentFileConfig::default_model(), "qwen3-coder:30b");
    }

    #[test]
    fn test_default_ollama_url_value() {
        assert_eq!(
            AgentFileConfig::default_ollama_url(),
            "http://localhost:11434"
        );
    }

    // ============== Edge Cases ==============

    #[test]
    fn test_mcp_config_unicode_server_name() {
        let json = r#"{
            "mcpServers": {
                "服务器": {
                    "command": "/bin/test"
                }
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let config = McpConfig::load_from_path(file.path()).unwrap();
        assert!(config.mcp_servers.contains_key("服务器"));
    }

    #[test]
    fn test_agent_config_special_chars_in_prompt() {
        let toml = r#"
[agent]
system_prompt = "Prompt with 'quotes' and \"escapes\" and \nnewlines"
"#;

        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        file.write_all(toml.as_bytes()).unwrap();

        let config = AgentFileConfig::load_from_path(file.path()).unwrap();
        assert!(config.agent.system_prompt.is_some());
    }

    #[test]
    fn test_agent_config_very_large_interval() {
        let toml = r#"
[monitor]
interval = 999999999
"#;

        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        file.write_all(toml.as_bytes()).unwrap();

        let config = AgentFileConfig::load_from_path(file.path()).unwrap();
        assert_eq!(config.monitor.interval, 999999999);
    }

    #[test]
    fn test_mcp_config_tier_boundaries() {
        let json = r#"{
            "mcpServers": {
                "tier1": {"command": "cmd", "tier": 1},
                "tier2": {"command": "cmd", "tier": 2},
                "tier3": {"command": "cmd", "tier": 3},
                "tier4": {"command": "cmd", "tier": 4}
            }
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let config = McpConfig::load_from_path(file.path()).unwrap();
        assert_eq!(config.mcp_servers["tier1"].tier, 1);
        assert_eq!(config.mcp_servers["tier2"].tier, 2);
        assert_eq!(config.mcp_servers["tier3"].tier, 3);
        assert_eq!(config.mcp_servers["tier4"].tier, 4);
    }

    #[test]
    fn test_mcp_config_extra_fields_ignored() {
        let json = r#"{
            "mcpServers": {
                "test": {
                    "command": "cmd",
                    "unknown_field": "ignored",
                    "another": 123
                }
            },
            "extraTopLevel": "also ignored"
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        // Should parse successfully, ignoring unknown fields
        let config = McpConfig::load_from_path(file.path()).unwrap();
        assert_eq!(config.mcp_servers["test"].command, "cmd");
    }
}
