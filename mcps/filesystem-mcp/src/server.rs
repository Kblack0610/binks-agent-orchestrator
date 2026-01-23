//! MCP Server implementation for sandboxed filesystem operations
//!
//! This module defines the main MCP server that exposes filesystem operations as tools.
//! Handler implementations are in the handlers module.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
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
        description = "List the contents of a directory. Returns file and directory entries with metadata."
    )]
    async fn list_dir(
        &self,
        Parameters(params): Parameters<ListDirParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::list_dir(&self.sandbox, &self.config, params).await
    }

    #[tool(
        description = "Search for files matching a glob pattern. Returns matching file paths."
    )]
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

    #[tool(description = "Create a directory. Uses recursive=true by default to create parent directories.")]
    async fn create_directory(
        &self,
        Parameters(params): Parameters<CreateDirParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::create_directory(&self.sandbox, params).await
    }

    #[tool(description = "List allowed directories that this server can access for reading and writing.")]
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
        Self::with_default_config().expect("Failed to create FilesystemMcpServer")
    }
}
