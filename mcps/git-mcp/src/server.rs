//! MCP Server implementation for Git operations
//!
//! This module defines the main MCP server that exposes git operations as tools.
//! Handler implementations are in the handlers/ module.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};

use crate::handlers;
use crate::params::*;

/// The Git MCP Server
#[derive(Clone)]
pub struct GitMcpServer {
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Tool Router - Each tool delegates to its handler
// ============================================================================

#[tool_router]
impl GitMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Get the status of a git repository, including staged, modified, and untracked files"
    )]
    async fn git_status(
        &self,
        Parameters(params): Parameters<StatusParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::status(params).await
    }

    #[tool(description = "Get the commit log of a git repository")]
    async fn git_log(
        &self,
        Parameters(params): Parameters<LogParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::log(params).await
    }

    #[tool(description = "Get the diff between two references or working directory")]
    async fn git_diff(
        &self,
        Parameters(params): Parameters<DiffParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::diff(params).await
    }

    #[tool(description = "Show details of a specific commit")]
    async fn git_show(
        &self,
        Parameters(params): Parameters<ShowParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::show(params).await
    }

    #[tool(description = "List branches in a git repository")]
    async fn git_branch_list(
        &self,
        Parameters(params): Parameters<BranchListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::branch_list(params).await
    }

    #[tool(description = "Show line-by-line authorship information (git blame) for a file")]
    async fn git_blame(
        &self,
        Parameters(params): Parameters<BlameParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::blame(params).await
    }

    #[tool(description = "Manage git stashes: list, show, save, pop, apply, or drop")]
    async fn git_stash(
        &self,
        Parameters(params): Parameters<StashParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::stash(params).await
    }

    #[tool(description = "List git remotes configured for the repository")]
    async fn git_remote_list(
        &self,
        Parameters(params): Parameters<RemoteListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::remote_list(params).await
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for GitMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Git MCP server providing local git repository operations using libgit2. \
                 Complements GitHub API tools with local repository access."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for GitMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
