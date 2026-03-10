//! Parameter structs for Unity MCP tools

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadLogParams {
    /// Number of lines to return from the end of the log (default: 100)
    #[schemars(description = "Number of lines to return from the end of the log (default: 100)")]
    pub lines: Option<usize>,

    /// Filter by log level: error, warning, or info
    #[schemars(description = "Filter by log level: error, warning, or info")]
    pub level: Option<String>,

    /// Regex pattern to filter log lines
    #[schemars(description = "Regex pattern to filter log lines")]
    pub pattern: Option<String>,

    /// Override the log file path (otherwise auto-detected)
    #[schemars(description = "Override the log file path (otherwise auto-detected)")]
    pub log_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LogErrorsParams {
    /// Override the log file path (otherwise auto-detected)
    #[schemars(description = "Override the log file path (otherwise auto-detected)")]
    pub log_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LogTailParams {
    /// Override the log file path (otherwise auto-detected)
    #[schemars(description = "Override the log file path (otherwise auto-detected)")]
    pub log_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ProjectInfoParams {
    /// Path to the Unity project directory (otherwise auto-detected from CWD)
    #[schemars(description = "Path to the Unity project directory (otherwise auto-detected from CWD)")]
    pub project_path: Option<String>,
}
