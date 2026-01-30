//! MCP Server implementation for task management
//!
//! This module defines the main MCP server that exposes task operations as tools.
//! Handler implementations are in the handlers module.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use std::path::PathBuf;

use crate::handlers;
use crate::params::*;
use crate::repository::TaskRepository;

/// The main Task MCP Server
#[derive(Clone)]
pub struct TaskMcpServer {
    repository: TaskRepository,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Tool Router - Each tool delegates to its handler
// ============================================================================

#[tool_router]
impl TaskMcpServer {
    pub fn new() -> Result<Self, anyhow::Error> {
        // Database path: ~/.binks/conversations.db (shared with agent)
        let db_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".binks")
            .join("conversations.db");

        let repository = TaskRepository::new(db_path)?;

        Ok(Self {
            repository,
            tool_router: Self::tool_router(),
        })
    }

    // ========================================================================
    // CRUD Operations
    // ========================================================================

    #[tool(description = "Create new task with metadata")]
    async fn create_task(
        &self,
        Parameters(params): Parameters<CreateTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::create_task(&self.repository, params).await
    }

    #[tool(description = "Fetch task by ID or prefix (min 8 chars)")]
    async fn get_task(
        &self,
        Parameters(params): Parameters<GetTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::get_task(&self.repository, params).await
    }

    #[tool(description = "Query tasks with filters (status, plan, priority)")]
    async fn list_tasks(
        &self,
        Parameters(params): Parameters<ListTasksParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::list_tasks(&self.repository, params).await
    }

    #[tool(description = "Update task fields (status, branch, PR URL)")]
    async fn update_task(
        &self,
        Parameters(params): Parameters<UpdateTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::update_task(&self.repository, params).await
    }

    // ========================================================================
    // Dependency Management
    // ========================================================================

    #[tool(description = "Create task → depends_on link")]
    async fn add_dependency(
        &self,
        Parameters(params): Parameters<AddDependencyParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::add_dependency(&self.repository, params).await
    }

    #[tool(description = "Get blocking/blocked tasks")]
    async fn list_dependencies(
        &self,
        Parameters(params): Parameters<ListDependenciesParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::list_dependencies(&self.repository, params).await
    }

    #[tool(description = "Verify if task is unblocked")]
    async fn check_blocking_tasks(
        &self,
        Parameters(params): Parameters<CheckBlockingTasksParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::check_blocking_tasks(&self.repository, params).await
    }

    // ========================================================================
    // Execution Tracking
    // ========================================================================

    #[tool(description = "Link task to run result")]
    async fn record_execution(
        &self,
        Parameters(params): Parameters<RecordExecutionParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::record_execution(&self.repository, params).await
    }

    #[tool(description = "Associate task with run ID")]
    async fn link_to_run(
        &self,
        Parameters(params): Parameters<LinkToRunParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::link_to_run(&self.repository, params).await
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    #[tool(description = "Filter tasks by pending/in_progress/completed")]
    async fn tasks_by_status(
        &self,
        Parameters(params): Parameters<TasksByStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::tasks_by_status(&self.repository, params).await
    }

    #[tool(description = "All tasks from plan source")]
    async fn tasks_by_plan(
        &self,
        Parameters(params): Parameters<TasksByPlanParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::tasks_by_plan(&self.repository, params).await
    }

    #[tool(description = "Atomic task acquisition (prevents races)")]
    async fn grab_next_task(
        &self,
        Parameters(params): Parameters<GrabNextTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::grab_next_task(&self.repository, params).await
    }

    // ========================================================================
    // Memory Integration
    // ========================================================================

    #[tool(description = "Create memory entity for task (optional)")]
    async fn sync_to_memory(
        &self,
        Parameters(params): Parameters<SyncToMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::sync_to_memory(&self.repository, params).await
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for TaskMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Task management MCP server with CRUD operations, dependency management, and execution tracking. \
                 Shares ~/.binks/conversations.db with the agent for task execution state. \
                 Integrates with memory-mcp for task knowledge and context."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for TaskMcpServer {
    fn default() -> Self {
        Self::new().expect("Failed to create TaskMcpServer")
    }
}
