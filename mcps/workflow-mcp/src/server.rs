//! Workflow MCP Server implementation

use crate::engine::ExecutionRegistry;
use crate::loader::{builtin_workflows, load_custom_workflows};
use crate::params::{
    ExecuteWorkflowParams, ExecutionStatusParams, ListWorkflowsParams, ResumeCheckpointParams,
};
use crate::types::{Workflow, WorkflowStep};
use mcp_common::{McpError, ResultExt};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Workflow MCP Server
#[derive(Clone)]
pub struct WorkflowMcpServer {
    /// MCP tool router
    tool_router: ToolRouter<Self>,

    /// Registry of active workflow executions
    pub executions: Arc<Mutex<ExecutionRegistry>>,

    /// Built-in workflows
    pub builtin_workflows: HashMap<String, Workflow>,

    /// Custom workflows directory
    pub custom_workflows_dir: Option<PathBuf>,
}

// ============================================================================
// Tool Router - Tools will be added in Phase 2.4
// ============================================================================

#[tool_router]
impl WorkflowMcpServer {
    /// Create a new workflow MCP server
    pub fn new() -> Self {
        let builtin = builtin_workflows();

        // Try to load custom workflows from ~/.binks/workflows/
        let custom_dir = dirs::home_dir().map(|h| h.join(".binks").join("workflows"));

        Self {
            tool_router: Self::tool_router(),
            executions: Arc::new(Mutex::new(ExecutionRegistry::new())),
            builtin_workflows: builtin,
            custom_workflows_dir: custom_dir,
        }
    }

    /// Get all available workflows (built-in + custom)
    pub fn get_all_workflows(&self) -> HashMap<String, Workflow> {
        let mut workflows = self.builtin_workflows.clone();

        if let Some(custom_dir) = &self.custom_workflows_dir {
            if custom_dir.exists() {
                match load_custom_workflows(custom_dir) {
                    Ok(custom) => workflows.extend(custom),
                    Err(e) => {
                        tracing::warn!("Failed to load custom workflows: {}", e);
                    }
                }
            }
        }

        workflows
    }

    /// Get a workflow by name
    pub fn get_workflow(&self, name: &str) -> Option<Workflow> {
        self.get_all_workflows().get(name).cloned()
    }

    // ========================================================================
    // MCP Tool Handlers
    // ========================================================================

    /// List all available workflows (built-in and custom)
    #[tool(description = "List all available workflows with their descriptions and steps")]
    async fn list_workflows(
        &self,
        Parameters(_params): Parameters<ListWorkflowsParams>,
    ) -> Result<CallToolResult, McpError> {
        let workflows = self.get_all_workflows();

        // Build response with workflow summaries
        let mut summary = Vec::new();
        for (name, workflow) in workflows.iter() {
            let steps_info: Vec<serde_json::Value> = workflow
                .steps
                .iter()
                .enumerate()
                .map(|(idx, step)| {
                    let step_info = match step {
                        WorkflowStep::Agent { name, task, model } => {
                            serde_json::json!({
                                "index": idx,
                                "type": "agent",
                                "agent": name,
                                "task_template": task,
                                "model": model,
                            })
                        }
                        WorkflowStep::Checkpoint { message, show } => {
                            serde_json::json!({
                                "index": idx,
                                "type": "checkpoint",
                                "message": message,
                                "show": show,
                            })
                        }
                        WorkflowStep::Parallel(_) => {
                            serde_json::json!({
                                "index": idx,
                                "type": "parallel",
                            })
                        }
                        WorkflowStep::Branch { .. } => {
                            serde_json::json!({
                                "index": idx,
                                "type": "branch",
                            })
                        }
                    };
                    step_info
                })
                .collect();

            summary.push(serde_json::json!({
                "name": name,
                "description": workflow.description,
                "step_count": workflow.steps.len(),
                "steps": steps_info,
            }));
        }

        let response = serde_json::json!({
            "workflows": summary,
            "total": workflows.len(),
        });

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&response).to_mcp_err()?,
        )]))
    }

    /// Execute a workflow with the given task and context
    #[tool(
        description = "Execute a workflow with the given task and context. Returns an execution ID for tracking."
    )]
    async fn execute_workflow(
        &self,
        Parameters(params): Parameters<ExecuteWorkflowParams>,
    ) -> Result<CallToolResult, McpError> {
        // Get the workflow by name
        let workflow = self.get_workflow(&params.workflow).ok_or_else(|| {
            McpError::invalid_params(format!("Workflow not found: {}", params.workflow), None)
        })?;

        // Create initial context with the task
        let mut context = params.context;
        context.insert("task".to_string(), params.task);

        // Create execution state
        let state = crate::engine::ExecutionState::new(workflow.name.clone(), context);
        let execution_id = state.id.clone();

        // Register the execution
        {
            let mut registry = self.executions.lock().map_err(|e| {
                McpError::internal_error(format!("Failed to lock execution registry: {}", e), None)
            })?;
            registry.register(state);
        }

        let response = serde_json::json!({
            "execution_id": execution_id,
            "workflow_name": workflow.name,
            "status": "running",
            "message": "Workflow execution started. Note: Actual step execution is not yet implemented."
        });

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&response).to_mcp_err()?,
        )]))
    }

    /// Get the status of a workflow execution
    #[tool(description = "Get the current status and progress of a workflow execution by ID")]
    async fn get_execution_status(
        &self,
        Parameters(params): Parameters<ExecutionStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let registry = self.executions.lock().map_err(|e| {
            McpError::internal_error(format!("Failed to lock execution registry: {}", e), None)
        })?;

        let state = registry
            .get(&params.execution_id)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let response = serde_json::json!({
            "execution_id": state.id,
            "workflow_name": state.workflow_name,
            "current_step": state.current_step,
            "status": format!("{:?}", state.status),
            "completed_steps": state.step_results.len(),
            "context": state.context,
        });

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&response).to_mcp_err()?,
        )]))
    }

    /// Resume a workflow execution from a checkpoint
    #[tool(
        description = "Resume a paused workflow execution from a checkpoint with user approval/rejection"
    )]
    async fn resume_from_checkpoint(
        &self,
        Parameters(params): Parameters<ResumeCheckpointParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut registry = self.executions.lock().map_err(|e| {
            McpError::internal_error(format!("Failed to lock execution registry: {}", e), None)
        })?;

        let state = registry
            .get_mut(&params.execution_id)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        // Add feedback to context if provided
        if let Some(feedback) = &params.feedback {
            state
                .context
                .insert("checkpoint_feedback".to_string(), feedback.clone());
        }

        // Add approval status to context
        state.context.insert(
            "checkpoint_approved".to_string(),
            params.approved.to_string(),
        );

        let response = serde_json::json!({
            "execution_id": state.id,
            "workflow_name": state.workflow_name,
            "approved": params.approved,
            "message": if params.approved {
                "Checkpoint approved. Workflow will continue. Note: Actual step execution is not yet implemented."
            } else {
                "Checkpoint rejected. Workflow execution stopped."
            }
        });

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&response).to_mcp_err()?,
        )]))
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for WorkflowMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Workflow MCP server providing workflow orchestration capabilities. \
                 Execute multi-step agent workflows with checkpoint support."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for WorkflowMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
