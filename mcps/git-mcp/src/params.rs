//! Parameter types for Git MCP tools

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatusParams {
    /// Path to the git repository (defaults to current directory)
    #[serde(default)]
    pub repo_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LogParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// Maximum number of commits to return (default: 10)
    #[serde(default)]
    pub limit: Option<usize>,
    /// Reference to start from (branch, tag, or commit)
    #[serde(default)]
    pub rev: Option<String>,
    /// Path filter (only commits affecting this path)
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiffParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// From reference (defaults to HEAD)
    #[serde(default)]
    pub from_ref: Option<String>,
    /// To reference (defaults to working directory)
    #[serde(default)]
    pub to_ref: Option<String>,
    /// Include file contents in diff (default: true)
    #[serde(default = "default_true")]
    pub include_patch: Option<bool>,
    /// Path filter
    #[serde(default)]
    pub path: Option<String>,
}

fn default_true() -> Option<bool> {
    Some(true)
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// Reference to show (commit, tag, etc.)
    pub rev: String,
    /// Include diff in output (default: false)
    #[serde(default)]
    pub include_diff: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BranchListParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// Include remote branches (default: false)
    #[serde(default)]
    pub include_remote: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlameParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// Path to the file to blame
    pub file_path: String,
    /// Starting line (1-indexed, default: 1)
    #[serde(default)]
    pub start_line: Option<usize>,
    /// Ending line (default: end of file)
    #[serde(default)]
    pub end_line: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StashParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// Action: "list", "show", "save", "pop", "apply", "drop"
    pub action: String,
    /// Stash index for show/pop/apply/drop (default: 0)
    #[serde(default)]
    pub index: Option<usize>,
    /// Message for save action
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoteListParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
}
