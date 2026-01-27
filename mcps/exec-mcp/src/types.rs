//! Type definitions for exec MCP

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Configuration Types
// ============================================================================

/// Configuration for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub commands: CommandConfig,
    #[serde(default)]
    pub timeouts: TimeoutConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
    #[serde(default)]
    pub environment: EnvConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            commands: CommandConfig::default(),
            timeouts: TimeoutConfig::default(),
            limits: LimitsConfig::default(),
            environment: EnvConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    /// Shell to use for executing commands
    #[serde(default = "default_shell")]
    pub shell: String,

    /// Regex patterns for allowed commands (empty = allow all not denied)
    #[serde(default)]
    pub allow_patterns: Vec<String>,

    /// Regex patterns for denied commands (always takes precedence over allow)
    #[serde(default = "default_deny_patterns")]
    pub deny_patterns: Vec<String>,

    /// Allowed working directories
    #[serde(default = "default_allowed_dirs")]
    pub allowed_dirs: Vec<String>,
}

fn default_shell() -> String {
    "/bin/bash".to_string()
}

fn default_deny_patterns() -> Vec<String> {
    vec![
        r"rm\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?/\s*$".to_string(),  // rm -rf /
        r"rm\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?/\s".to_string(),      // rm -rf / <more>
        r"mkfs\.".to_string(),
        r"^\s*dd\s+.*of=/dev/".to_string(),
        r":\(\)\{.*\|.*&.*\}".to_string(),                       // fork bomb
        r"^\s*(shutdown|reboot|halt|poweroff)\b".to_string(),
        r"^\s*chmod\s+(-[a-zA-Z]*)?\s*777\s+/".to_string(),
        r">\s*/dev/sd[a-z]".to_string(),
        r"^\s*:\(\)\{ :\|:& \};:".to_string(),                    // fork bomb variant
    ]
}

fn default_allowed_dirs() -> Vec<String> {
    vec![
        "~/dev".to_string(),
        "~/projects".to_string(),
        "/tmp".to_string(),
    ]
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            shell: default_shell(),
            allow_patterns: Vec::new(),
            deny_patterns: default_deny_patterns(),
            allowed_dirs: default_allowed_dirs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Default timeout in seconds
    #[serde(default = "default_timeout")]
    pub default_secs: u64,
    /// Maximum timeout in seconds (hard cap)
    #[serde(default = "default_max_timeout")]
    pub max_secs: u64,
}

fn default_timeout() -> u64 {
    30
}

fn default_max_timeout() -> u64 {
    300
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_secs: default_timeout(),
            max_secs: default_max_timeout(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    /// Maximum output size per stream (stdout/stderr) in bytes
    #[serde(default = "default_max_output")]
    pub max_output_bytes: usize,
}

fn default_max_output() -> usize {
    1024 * 1024 // 1MB
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_output_bytes: default_max_output(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvConfig {
    /// Environment variables to set
    #[serde(default)]
    pub set: std::collections::HashMap<String, String>,
    /// Environment variables to remove
    #[serde(default)]
    pub remove: Vec<String>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for command execution
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandOutput {
    pub command: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub truncated: bool,
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum ExecError {
    #[error("Command denied: {0}")]
    CommandDenied(String),

    #[error("Working directory not allowed: {0}")]
    DirNotAllowed(String),

    #[error("Command timed out after {0}s")]
    Timeout(u64),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Config error: {0}")]
    ConfigError(String),
}
