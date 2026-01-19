//! MCP Server implementation for sandboxed filesystem operations

use chrono::{DateTime, Utc};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::sandbox::Sandbox;
use crate::types::{
    Config, DeleteFileResponse, FileEntry, FileInfoResponse, FsError, ListDirResponse,
    MoveFileResponse, ReadFileResponse, SearchFilesResponse, WriteFileResponse,
};

/// The Filesystem MCP Server
#[derive(Clone)]
pub struct FilesystemMcpServer {
    sandbox: Sandbox,
    config: Config,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Parameter Types
// ============================================================================

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
    #[serde(default)]
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
    #[serde(default)]
    pub recursive: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateDirParams {
    #[schemars(description = "Path of the directory to create")]
    pub path: String,

    #[schemars(description = "Create parent directories as needed (default: true)")]
    #[serde(default = "default_true")]
    pub recursive: bool,
}

fn default_true() -> bool {
    true
}

// ============================================================================
// Helper Functions
// ============================================================================

fn fs_error_to_mcp(err: FsError) -> McpError {
    match &err {
        FsError::AccessDenied(_) | FsError::PathTraversal(_) => {
            McpError::invalid_request(err.to_string(), None)
        }
        FsError::NotFound(_) => McpError::invalid_params(err.to_string(), None),
        FsError::FileTooLarge { .. } => McpError::invalid_request(err.to_string(), None),
        _ => McpError::internal_error(err.to_string(), None),
    }
}

async fn get_file_entry(path: &Path) -> Result<FileEntry, std::io::Error> {
    let metadata = fs::metadata(path).await?;
    let modified: Option<DateTime<Utc>> = metadata.modified().ok().map(|t| t.into());

    Ok(FileEntry {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        path: path.display().to_string(),
        entry_type: if metadata.is_dir() {
            "directory".to_string()
        } else {
            "file".to_string()
        },
        size: if metadata.is_file() {
            Some(metadata.len())
        } else {
            None
        },
        modified,
    })
}

// ============================================================================
// Tool Router Implementation
// ============================================================================

#[tool_router]
impl FilesystemMcpServer {
    pub fn new(config: Config) -> Result<Self, FsError> {
        let sandbox = Sandbox::new(&config)?;

        Ok(Self {
            sandbox,
            config,
            tool_router: Self::tool_router(),
        })
    }

    pub fn with_default_config() -> Result<Self, FsError> {
        Self::new(Config::default())
    }

    #[tool(
        description = "Read the complete contents of a file. Returns the file content as a string."
    )]
    async fn read_file(
        &self,
        Parameters(params): Parameters<ReadFileParams>,
    ) -> Result<CallToolResult, McpError> {
        let canonical = self.sandbox.validate_read(&params.path).map_err(fs_error_to_mcp)?;

        // Check file size before reading
        let metadata = fs::metadata(&canonical)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if metadata.len() > self.config.limits.max_file_size as u64 {
            return Err(fs_error_to_mcp(FsError::FileTooLarge {
                size: metadata.len(),
                max: self.config.limits.max_file_size,
            }));
        }

        let content = fs::read_to_string(&canonical)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let response = ReadFileResponse {
            path: canonical.display().to_string(),
            content,
            size: metadata.len(),
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Write content to a file. Creates the file if it doesn't exist, overwrites if it does."
    )]
    async fn write_file(
        &self,
        Parameters(params): Parameters<WriteFileParams>,
    ) -> Result<CallToolResult, McpError> {
        let canonical = self.sandbox.validate_write(&params.path).map_err(fs_error_to_mcp)?;

        // Check content size
        if params.content.len() > self.config.limits.max_file_size {
            return Err(fs_error_to_mcp(FsError::FileTooLarge {
                size: params.content.len() as u64,
                max: self.config.limits.max_file_size,
            }));
        }

        let mut file = fs::File::create(&canonical)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        file.write_all(params.content.as_bytes())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let response = WriteFileResponse {
            path: canonical.display().to_string(),
            success: true,
            bytes_written: params.content.len(),
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "List the contents of a directory. Returns file and directory entries with metadata."
    )]
    async fn list_dir(
        &self,
        Parameters(params): Parameters<ListDirParams>,
    ) -> Result<CallToolResult, McpError> {
        let canonical = self.sandbox.validate_read(&params.path).map_err(fs_error_to_mcp)?;

        let mut entries = Vec::new();
        let mut count = 0;

        if params.recursive {
            // Recursive listing
            let mut stack = vec![canonical.clone()];
            while let Some(dir) = stack.pop() {
                if count >= self.config.limits.max_files_per_list {
                    break;
                }

                let mut read_dir = fs::read_dir(&dir)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                while let Some(entry) = read_dir
                    .next_entry()
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
                {
                    if count >= self.config.limits.max_files_per_list {
                        break;
                    }

                    let path = entry.path();

                    // Verify path is still allowed
                    if self.sandbox.check_read(&path).is_err() {
                        continue;
                    }

                    if let Ok(file_entry) = get_file_entry(&path).await {
                        if file_entry.entry_type == "directory" {
                            stack.push(path);
                        }
                        entries.push(file_entry);
                        count += 1;
                    }
                }
            }
        } else {
            // Non-recursive listing
            let mut read_dir = fs::read_dir(&canonical)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            while let Some(entry) = read_dir
                .next_entry()
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
            {
                if count >= self.config.limits.max_files_per_list {
                    break;
                }

                let path = entry.path();
                if let Ok(file_entry) = get_file_entry(&path).await {
                    entries.push(file_entry);
                    count += 1;
                }
            }
        }

        let response = ListDirResponse {
            path: canonical.display().to_string(),
            entries,
            total_count: count,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Search for files matching a glob pattern. Returns matching file paths."
    )]
    async fn search_files(
        &self,
        Parameters(params): Parameters<SearchFilesParams>,
    ) -> Result<CallToolResult, McpError> {
        let canonical = self.sandbox.validate_read(&params.path).map_err(fs_error_to_mcp)?;

        // Combine base path with pattern
        let full_pattern = canonical.join(&params.pattern);
        let pattern_str = full_pattern.display().to_string();

        let mut matches = Vec::new();
        let mut count = 0;

        // Use glob to find matches
        for entry in glob::glob(&pattern_str)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?
        {
            if count >= self.config.limits.max_files_per_list {
                break;
            }

            if let Ok(path) = entry {
                // Verify path is allowed
                if self.sandbox.check_read(&path).is_ok() {
                    matches.push(path.display().to_string());
                    count += 1;
                }
            }
        }

        let response = SearchFilesResponse {
            pattern: params.pattern,
            base_path: canonical.display().to_string(),
            matches,
            total_count: count,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Get detailed information about a file or directory including size, modification time, and type."
    )]
    async fn file_info(
        &self,
        Parameters(params): Parameters<FileInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        let canonical = self.sandbox.validate_read(&params.path).map_err(fs_error_to_mcp)?;

        let response = if canonical.exists() {
            let metadata = fs::metadata(&canonical)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            let modified: Option<DateTime<Utc>> = metadata.modified().ok().map(|t| t.into());
            let created: Option<DateTime<Utc>> = metadata.created().ok().map(|t| t.into());

            FileInfoResponse {
                path: canonical.display().to_string(),
                exists: true,
                entry_type: Some(if metadata.is_dir() {
                    "directory".to_string()
                } else if metadata.is_symlink() {
                    "symlink".to_string()
                } else {
                    "file".to_string()
                }),
                size: if metadata.is_file() {
                    Some(metadata.len())
                } else {
                    None
                },
                modified,
                created,
                readonly: Some(metadata.permissions().readonly()),
            }
        } else {
            FileInfoResponse {
                path: canonical.display().to_string(),
                exists: false,
                entry_type: None,
                size: None,
                modified: None,
                created: None,
                readonly: None,
            }
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Move or rename a file or directory. Both source and destination must be within allowed paths."
    )]
    async fn move_file(
        &self,
        Parameters(params): Parameters<MoveFileParams>,
    ) -> Result<CallToolResult, McpError> {
        // Source must be readable, destination must be writable
        let src_canonical = self.sandbox.validate_read(&params.src).map_err(fs_error_to_mcp)?;
        let dst_canonical = self.sandbox.validate_write(&params.dst).map_err(fs_error_to_mcp)?;

        fs::rename(&src_canonical, &dst_canonical)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let response = MoveFileResponse {
            src: src_canonical.display().to_string(),
            dst: dst_canonical.display().to_string(),
            success: true,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Delete a file or directory. Use recursive=true to delete non-empty directories."
    )]
    async fn delete_file(
        &self,
        Parameters(params): Parameters<DeleteFileParams>,
    ) -> Result<CallToolResult, McpError> {
        let canonical = self.sandbox.validate_write(&params.path).map_err(fs_error_to_mcp)?;

        if !canonical.exists() {
            return Err(fs_error_to_mcp(FsError::NotFound(
                canonical.display().to_string(),
            )));
        }

        let metadata = fs::metadata(&canonical)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if metadata.is_dir() {
            if params.recursive {
                fs::remove_dir_all(&canonical)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            } else {
                fs::remove_dir(&canonical)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            }
        } else {
            fs::remove_file(&canonical)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        let response = DeleteFileResponse {
            path: canonical.display().to_string(),
            deleted: true,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Create a directory. Uses recursive=true by default to create parent directories.")]
    async fn create_directory(
        &self,
        Parameters(params): Parameters<CreateDirParams>,
    ) -> Result<CallToolResult, McpError> {
        let canonical = self.sandbox.validate_write(&params.path).map_err(fs_error_to_mcp)?;

        if params.recursive {
            fs::create_dir_all(&canonical)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        } else {
            fs::create_dir(&canonical)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        let response = serde_json::json!({
            "path": canonical.display().to_string(),
            "created": true
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List allowed directories that this server can access for reading and writing.")]
    async fn list_allowed_directories(&self) -> Result<CallToolResult, McpError> {
        let response = serde_json::json!({
            "read_paths": self.sandbox.allowed_read_paths(),
            "write_paths": self.sandbox.allowed_write_paths()
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for FilesystemMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Sandboxed filesystem MCP server with security controls. \
                 Operations are restricted to configured allowed directories. \
                 Use list_allowed_directories to see what paths are accessible."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for FilesystemMcpServer {
    fn default() -> Self {
        Self::with_default_config().expect("Failed to create FilesystemMcpServer")
    }
}
