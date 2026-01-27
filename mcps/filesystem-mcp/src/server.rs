//! MCP Server implementation for sandboxed filesystem operations
//!
//! This module defines the main MCP server that exposes filesystem operations as tools.
//! Handler implementations are in the handlers module.

use std::path::PathBuf;

use mcp_common::{CallToolResult, McpError};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::handlers;
use crate::params::*;
use crate::sandbox::Sandbox;
use crate::types::{Config, FsError};

/// The Filesystem MCP Server
#[derive(Clone)]
pub struct FilesystemMcpServer {
    sandbox: Sandbox,
    config: Config,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Tool Router - Each tool delegates to its handler
// ============================================================================

#[tool_router]
impl FilesystemMcpServer {
    /// Create a new server, loading config from standard locations
    ///
    /// Config is searched in order:
    /// 1. `FS_CONFIG_PATH` env var
    /// 2. `~/.binks/filesystem.toml`
    /// 3. `./filesystem-mcp.toml`
    /// 4. `$XDG_CONFIG_HOME/filesystem-mcp/config.toml`
    /// 5. `~/.filesystem-mcp.toml`
    /// 6. Default config if none found
    pub fn new() -> Self {
        Self::with_config(Self::load_config()).expect("Failed to create FilesystemMcpServer")
    }

    /// Create a new server with explicit config
    pub fn with_config(config: Config) -> Result<Self, FsError> {
        let sandbox = Sandbox::new(&config)?;

        Ok(Self {
            sandbox,
            config,
            tool_router: Self::tool_router(),
        })
    }

    /// Load config from standard file locations
    fn load_config() -> Config {
        // 1. Check FS_CONFIG_PATH env var first
        if let Ok(env_path) = std::env::var("FS_CONFIG_PATH") {
            let path = PathBuf::from(&env_path);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    match toml::from_str::<Config>(&content) {
                        Ok(config) => {
                            tracing::info!("Loaded config from FS_CONFIG_PATH={}", path.display());
                            return config;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse config from FS_CONFIG_PATH={}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            } else {
                tracing::warn!("FS_CONFIG_PATH={} does not exist", env_path);
            }
        }

        // 2-5. Check standard file locations
        let mut config_paths = Vec::new();

        // 2. ~/.binks/filesystem.toml (project convention)
        if let Some(home) = dirs::home_dir() {
            config_paths.push(home.join(".binks").join("filesystem.toml"));
        }

        // 3. ./filesystem-mcp.toml (local override)
        config_paths.push(PathBuf::from("filesystem-mcp.toml"));

        // 4. $XDG_CONFIG_HOME/filesystem-mcp/config.toml
        if let Some(config_dir) = dirs::config_dir() {
            config_paths.push(config_dir.join("filesystem-mcp").join("config.toml"));
        }

        // 5. ~/.filesystem-mcp.toml
        if let Some(home) = dirs::home_dir() {
            config_paths.push(home.join(".filesystem-mcp.toml"));
        }

        for path in config_paths {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    match toml::from_str::<Config>(&content) {
                        Ok(config) => {
                            tracing::info!("Loaded config from {}", path.display());
                            return config;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse config {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        // 6. Default config
        tracing::info!("Using default configuration");
        Config::default()
    }

    #[tool(
        description = "Read the complete contents of a file. Returns the file content as a string."
    )]
    async fn read_file(
        &self,
        Parameters(params): Parameters<ReadFileParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::read_file(&self.sandbox, &self.config, params).await
    }

    #[tool(
        description = "Write content to a file. Creates the file if it doesn't exist, overwrites if it does."
    )]
    async fn write_file(
        &self,
        Parameters(params): Parameters<WriteFileParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::write_file(&self.sandbox, &self.config, params).await
    }

    #[tool(
        description = "Edit a file by replacing an exact string match. If old_string is empty, new_string is prepended. If new_string is empty, old_string is deleted. The match must be unique."
    )]
    async fn edit_file(
        &self,
        Parameters(params): Parameters<EditFileParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::edit_file(&self.sandbox, &self.config, params).await
    }

    #[tool(
        description = "List the contents of a directory. Returns file and directory entries with metadata."
    )]
    async fn list_dir(
        &self,
        Parameters(params): Parameters<ListDirParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::list_dir(&self.sandbox, &self.config, params).await
    }

    #[tool(description = "Search for files matching a glob pattern. Returns matching file paths.")]
    async fn search_files(
        &self,
        Parameters(params): Parameters<SearchFilesParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::search_files(&self.sandbox, &self.config, params).await
    }

    #[tool(
        description = "Get detailed information about a file or directory including size, modification time, and type."
    )]
    async fn file_info(
        &self,
        Parameters(params): Parameters<FileInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::file_info(&self.sandbox, params).await
    }

    #[tool(
        description = "Move or rename a file or directory. Both source and destination must be within allowed paths."
    )]
    async fn move_file(
        &self,
        Parameters(params): Parameters<MoveFileParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::move_file(&self.sandbox, params).await
    }

    #[tool(
        description = "Delete a file or directory. Use recursive=true to delete non-empty directories."
    )]
    async fn delete_file(
        &self,
        Parameters(params): Parameters<DeleteFileParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::delete_file(&self.sandbox, params).await
    }

    #[tool(
        description = "Create a directory. Uses recursive=true by default to create parent directories."
    )]
    async fn create_directory(
        &self,
        Parameters(params): Parameters<CreateDirParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::create_directory(&self.sandbox, params).await
    }

    #[tool(
        description = "Read multiple files simultaneously. Each file is read independently; failures for individual files don't affect others. Returns results for all requested files."
    )]
    async fn read_multiple_files(
        &self,
        Parameters(params): Parameters<ReadMultipleFilesParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::read_multiple_files(&self.sandbox, &self.config, params).await
    }

    #[tool(
        description = "Get a recursive directory tree structure as nested JSON. Returns file names, types, and sizes organized hierarchically."
    )]
    async fn directory_tree(
        &self,
        Parameters(params): Parameters<DirectoryTreeParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::directory_tree(&self.sandbox, &self.config, params).await
    }

    #[tool(
        description = "List allowed directories that this server can access for reading and writing."
    )]
    async fn list_allowed_directories(&self) -> Result<CallToolResult, McpError> {
        handlers::list_allowed_directories(&self.sandbox).await
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
        Self::new()
    }
}
