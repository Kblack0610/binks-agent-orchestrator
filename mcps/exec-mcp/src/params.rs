//! Parameter types for Exec MCP tools

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunCommandParams {
    #[schemars(description = "The shell command to execute")]
    pub command: String,

    #[schemars(description = "Working directory (optional, defaults to home directory)")]
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunCommandWithTimeoutParams {
    #[schemars(description = "The shell command to execute")]
    pub command: String,

    #[schemars(description = "Timeout in seconds (clamped to server max)")]
    pub timeout_secs: u64,

    #[schemars(description = "Working directory (optional, defaults to home directory)")]
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunScriptParams {
    #[schemars(description = "Multi-line script to execute via the configured shell")]
    pub script: String,

    #[schemars(description = "Working directory (optional, defaults to home directory)")]
    #[serde(default)]
    pub cwd: Option<String>,

    #[schemars(description = "Timeout in seconds (optional, uses default if not provided)")]
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}
