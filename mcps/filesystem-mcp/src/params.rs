//! Parameter types for Filesystem MCP tools

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileParams {
    #[schemars(description = "Path to the file to read")]
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WriteFileParams {
    #[schemars(description = "Path to the file to write")]
    pub path: String,

    #[schemars(description = "Content to write to the file")]
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListDirParams {
    #[schemars(description = "Path to the directory to list")]
    pub path: String,

    #[schemars(description = "Include files recursively (default: false)")]
    #[serde(default, deserialize_with = "crate::types::deserialize_lenient_bool")]
    pub recursive: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchFilesParams {
    #[schemars(description = "Base path to search from")]
    pub path: String,

    #[schemars(description = "Glob pattern to match (e.g., '*.rs', '**/*.json')")]
    pub pattern: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FileInfoParams {
    #[schemars(description = "Path to the file or directory")]
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MoveFileParams {
    #[schemars(description = "Source path")]
    pub src: String,

    #[schemars(description = "Destination path")]
    pub dst: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeleteFileParams {
    #[schemars(description = "Path to the file or directory to delete")]
    pub path: String,

    #[schemars(description = "Recursively delete directories (default: false)")]
    #[serde(default, deserialize_with = "crate::types::deserialize_lenient_bool")]
    pub recursive: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateDirParams {
    #[schemars(description = "Path of the directory to create")]
    pub path: String,

    #[schemars(description = "Create parent directories as needed (default: true)")]
    #[serde(default = "default_true", deserialize_with = "crate::types::deserialize_lenient_bool")]
    pub recursive: bool,
}

fn default_true() -> bool {
    true
}
