//! Configuration handling

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Poll interval in seconds
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,

    /// Auto-hide after seconds (0 = never)
    #[serde(default)]
    pub auto_hide_after: u64,

    /// Terminal command to open agents
    #[serde(default = "default_terminal")]
    pub terminal: String,

    /// Agent patterns to match in tmux
    #[serde(default = "default_agent_patterns")]
    pub agent_patterns: Vec<String>,
}

fn default_poll_interval() -> u64 {
    2
}

fn default_terminal() -> String {
    "kitty --single-instance -e".to_string()
}

fn default_agent_patterns() -> Vec<String> {
    vec![
        "claude".to_string(),
        "claude-real".to_string(),
        "aider".to_string(),
        "opencode".to_string(),
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            poll_interval: default_poll_interval(),
            auto_hide_after: 0,
            terminal: default_terminal(),
            agent_patterns: default_agent_patterns(),
        }
    }
}

impl Config {
    /// Load config from file or return defaults
    pub fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Get config file path
    pub fn config_path() -> PathBuf {
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs_next::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/"))
                    .join(".config")
            });
        config_dir.join("agent-panel").join("config.toml")
    }
}
