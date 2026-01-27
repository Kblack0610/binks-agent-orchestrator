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
    #[serde(
        default = "default_true",
        deserialize_with = "crate::types::deserialize_lenient_bool"
    )]
    pub recursive: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EditFileParams {
    #[schemars(description = "Path to the file to edit")]
    pub path: String,

    #[schemars(
        description = "The exact text to find and replace. If empty, new_string is prepended to the file."
    )]
    pub old_string: String,

    #[schemars(
        description = "The replacement text. If empty, the matched old_string is deleted."
    )]
    pub new_string: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadMultipleFilesParams {
    #[schemars(description = "Array of file paths to read. Each file is read independently; failures for individual files don't affect others.")]
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DirectoryTreeParams {
    #[schemars(description = "Path to the root directory for the tree")]
    pub path: String,

    #[schemars(description = "Maximum depth to traverse (default: 3, max: 10)")]
    pub depth: Option<u32>,
}

fn default_true() -> bool {
    true
}
