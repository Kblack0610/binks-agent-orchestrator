//! MCP Server implementation
//!
//! This module defines the main MCP server that exposes Linear CLI
//! operations as tools. Handler implementations are in the handlers/ module.
//!
//! # Feature-gated Tool Groups
//!
//! Issue tools are always available. Other tool groups require
//! Cargo feature flags:
//! - `teams` - Team listing and member lookup
//! - `projects` - Project listing
//! - `documents` - Document listing and viewing
//! - `full` - Enables all of the above

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};

use crate::handlers;
use crate::params::*;

/// The main Linear CLI MCP Server
#[derive(Clone)]
pub struct LinearCliMcpServer {
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Issue Tools (always available)
// ============================================================================

#[tool_router]
impl LinearCliMcpServer {
    #[tool(description = "List Linear issues with optional filters for state and sort order")]
    async fn linear_issue_list(
        &self,
        Parameters(params): Parameters<IssueListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_list(params).await
    }

    #[tool(
        description = "View a specific Linear issue. If no issue ID is provided, uses the current git branch"
    )]
    async fn linear_issue_view(
        &self,
        Parameters(params): Parameters<IssueViewParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_view(params).await
    }

    #[tool(description = "Create a new Linear issue with a title and optional description")]
    async fn linear_issue_create(
        &self,
        Parameters(params): Parameters<IssueCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_create(params).await
    }

    #[tool(
        description = "Start working on a Linear issue - changes status to started and creates a git branch"
    )]
    async fn linear_issue_start(
        &self,
        Parameters(params): Parameters<IssueStartParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_start(params).await
    }

    #[tool(description = "Add a comment to a Linear issue")]
    async fn linear_issue_comment_add(
        &self,
        Parameters(params): Parameters<IssueCommentAddParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_comment_add(params).await
    }

    #[tool(
        description = "Get the Linear issue ID associated with the current git branch"
    )]
    async fn linear_issue_id(
        &self,
        Parameters(_): Parameters<EmptyParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_id().await
    }
}

// ============================================================================
// Team Tools (requires "teams" feature)
// ============================================================================

#[cfg(feature = "teams")]
#[tool_router(router = team_tool_router)]
impl LinearCliMcpServer {
    #[tool(description = "List all Linear teams in the workspace")]
    async fn linear_team_list(
        &self,
        Parameters(_): Parameters<EmptyParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::team_list().await
    }

    #[tool(description = "List members of the current Linear team")]
    async fn linear_team_members(
        &self,
        Parameters(_): Parameters<EmptyParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::team_members().await
    }
}

// ============================================================================
// Project Tools (requires "projects" feature)
// ============================================================================

#[cfg(feature = "projects")]
#[tool_router(router = project_tool_router)]
impl LinearCliMcpServer {
    #[tool(description = "List all Linear projects")]
    async fn linear_project_list(
        &self,
        Parameters(_): Parameters<EmptyParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::project_list().await
    }
}

// ============================================================================
// Document Tools (requires "documents" feature)
// ============================================================================

#[cfg(feature = "documents")]
#[tool_router(router = document_tool_router)]
impl LinearCliMcpServer {
    #[tool(description = "List Linear documents with optional project or issue filter")]
    async fn linear_document_list(
        &self,
        Parameters(params): Parameters<DocumentListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::document_list(params).await
    }

    #[tool(description = "View a Linear document by its slug identifier")]
    async fn linear_document_view(
        &self,
        Parameters(params): Parameters<DocumentViewParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::document_view(params).await
    }
}

// ============================================================================
// Router Composition & Server Initialization
// ============================================================================

impl LinearCliMcpServer {
    pub fn new() -> Self {
        let router = Self::tool_router();

        #[cfg(feature = "teams")]
        let router = router + Self::team_tool_router();

        #[cfg(feature = "projects")]
        let router = router + Self::project_tool_router();

        #[cfg(feature = "documents")]
        let router = router + Self::document_tool_router();

        Self {
            tool_router: router,
        }
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for LinearCliMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Linear CLI MCP Server - provides tools for interacting with Linear \
                 issues, teams, projects, and documents using the linear CLI \
                 (schpet/linear-cli). Requires the linear CLI to be installed \
                 and authenticated."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for LinearCliMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
