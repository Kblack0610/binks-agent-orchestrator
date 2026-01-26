//! Type definitions for filesystem MCP

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

// ============================================================================
// Lenient Boolean Parsing (handles "true"/"false" strings from weak LLMs)
// ============================================================================

/// Deserialize a boolean that can be either a real bool or a string "true"/"false"
pub fn deserialize_lenient_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct LenientBoolVisitor;

    impl<'de> Visitor<'de> for LenientBoolVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a boolean or string 'true'/'false'")
        }

        fn visit_bool<E>(self, value: bool) -> Result<bool, E> {
            Ok(value)
        }

        fn visit_str<E>(self, value: &str) -> Result<bool, E>
        where
            E: de::Error,
        {
            match value.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok(true),
                "false" | "0" | "no" | "" => Ok(false),
                _ => Err(de::Error::custom(format!(
                    "invalid boolean string: '{}' (expected 'true' or 'false')",
                    value
                ))),
            }
        }

        fn visit_i64<E>(self, value: i64) -> Result<bool, E> {
            Ok(value != 0)
        }

        fn visit_u64<E>(self, value: u64) -> Result<bool, E> {
            Ok(value != 0)
        }
    }

    deserializer.deserialize_any(LenientBoolVisitor)
}

/// Deserialize an optional boolean with lenient parsing
#[allow(dead_code)]
pub fn deserialize_lenient_bool_opt<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct LenientBoolOptVisitor;

    impl<'de> Visitor<'de> for LenientBoolOptVisitor {
        type Value = Option<bool>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null, a boolean, or string 'true'/'false'")
        }

        fn visit_none<E>(self) -> Result<Option<bool>, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Option<bool>, E> {
            Ok(None)
        }

        fn visit_bool<E>(self, value: bool) -> Result<Option<bool>, E> {
            Ok(Some(value))
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<bool>, E>
        where
            E: de::Error,
        {
            match value.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok(Some(true)),
                "false" | "0" | "no" | "" => Ok(Some(false)),
                "null" | "none" => Ok(None),
                _ => Err(de::Error::custom(format!(
                    "invalid boolean string: '{}' (expected 'true' or 'false')",
                    value
                ))),
            }
        }

        fn visit_i64<E>(self, value: i64) -> Result<Option<bool>, E> {
            Ok(Some(value != 0))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Option<bool>, E> {
            Ok(Some(value != 0))
        }
    }

    deserializer.deserialize_any(LenientBoolOptVisitor)
}

// ============================================================================
// Configuration Types
// ============================================================================

/// Configuration for filesystem access
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub paths: PathConfig,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub safety: SafetyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    /// Directories the agent can read from
    #[serde(default = "default_read_paths")]
    pub read: Vec<String>,
    /// Directories the agent can write to (subset of read)
    #[serde(default = "default_write_paths")]
    pub write: Vec<String>,
    /// Directories never accessible
    #[serde(default = "default_deny_paths")]
    pub deny: Vec<String>,
}

fn default_read_paths() -> Vec<String> {
    vec!["~".to_string(), "/tmp".to_string()]
}

fn default_write_paths() -> Vec<String> {
    vec![
        "~/projects".to_string(),
        "~/dev".to_string(),
        "/tmp".to_string(),
    ]
}

fn default_deny_paths() -> Vec<String> {
    vec![
        "~/.ssh".to_string(),
        "~/.gnupg".to_string(),
        "~/.aws".to_string(),
        "~/.config/gh".to_string(),
    ]
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            read: default_read_paths(),
            write: default_write_paths(),
            deny: default_deny_paths(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    /// Maximum file size in bytes
    #[serde(default = "default_max_file_size")]
    pub max_file_size: usize,
    /// Maximum files per list operation
    #[serde(default = "default_max_files_per_list")]
    pub max_files_per_list: usize,
    /// Maximum search depth
    #[serde(default = "default_max_search_depth")]
    pub max_search_depth: usize,
}

fn default_max_file_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

fn default_max_files_per_list() -> usize {
    1000
}

fn default_max_search_depth() -> usize {
    10
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_size: default_max_file_size(),
            max_files_per_list: default_max_files_per_list(),
            max_search_depth: default_max_search_depth(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SafetyConfig {
    #[serde(default)]
    pub confirm_delete: bool,
    #[serde(default)]
    pub confirm_overwrite: bool,
    #[serde(default)]
    pub backup_on_overwrite: bool,
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for read_file operation
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadFileResponse {
    pub path: String,
    pub content: String,
    pub size: u64,
}

/// Response for write_file operation
#[derive(Debug, Serialize, Deserialize)]
pub struct WriteFileResponse {
    pub path: String,
    pub success: bool,
    pub bytes_written: usize,
}

/// Response for delete_file operation
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteFileResponse {
    pub path: String,
    pub deleted: bool,
}

/// Response for move_file operation
#[derive(Debug, Serialize, Deserialize)]
pub struct MoveFileResponse {
    pub src: String,
    pub dst: String,
    pub success: bool,
}

/// File or directory entry
#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: String, // "file" or "directory"
    pub size: Option<u64>,
    pub modified: Option<DateTime<Utc>>,
}

/// Response for list_dir operation
#[derive(Debug, Serialize, Deserialize)]
pub struct ListDirResponse {
    pub path: String,
    pub entries: Vec<FileEntry>,
    pub total_count: usize,
}

/// Response for search_files operation
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchFilesResponse {
    pub pattern: String,
    pub base_path: String,
    pub matches: Vec<String>,
    pub total_count: usize,
}

/// Response for file_info operation
#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfoResponse {
    pub path: String,
    pub exists: bool,
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub size: Option<u64>,
    pub modified: Option<DateTime<Utc>>,
    pub created: Option<DateTime<Utc>>,
    pub readonly: Option<bool>,
}

// ============================================================================
// Error Types
// ============================================================================

use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum FsError {
    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Path traversal attempt: {0}")]
    PathTraversal(String),

    #[error("File too large: {size} bytes (max {max})")]
    FileTooLarge { size: u64, max: usize },

    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Config error: {0}")]
    ConfigError(String),
}

pub type FsResult<T> = Result<T, FsError>;
