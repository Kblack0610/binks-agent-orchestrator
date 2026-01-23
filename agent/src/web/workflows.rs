//! Workflow API handlers
//!
//! Provides REST endpoints for orchestrator workflow management:
//! - List/show workflows and agents
//! - Start workflow runs (async)
//! - Poll for workflow status
//! - Submit checkpoint responses

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::state::AppState;
use crate::orchestrator::{
    AgentRegistry, EngineConfig, WorkflowEngine,
    workflow::{WorkflowStep, WorkflowStatus},
};

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    fn new(msg: impl Into<String>) -> Self {
        Self { error: msg.into() }
    }
}

// ============================================================================
// Response types
// ============================================================================

/// Workflow summary for listing
#[derive(Debug, Serialize)]
pub struct WorkflowSummary {
    pub name: String,
    pub description: String,
    pub is_custom: bool,
}

/// Workflow detail
#[derive(Debug, Serialize)]
pub struct WorkflowDetail {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStepInfo>,
}

/// Workflow step info
#[derive(Debug, Serialize)]
pub struct WorkflowStepInfo {
    pub step_type: String,
    pub agent_name: Option<String>,
    pub task_template: Option<String>,
    pub checkpoint_message: Option<String>,
}

/// Agent summary
#[derive(Debug, Serialize)]
pub struct AgentSummary {
    pub name: String,
    pub display_name: String,
    pub model: String,
    pub temperature: f32,
    pub tools: Vec<String>,
    pub can_handoff_to: Vec<String>,
}

/// Workflow run info
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowRunInfo {
    pub id: String,
    pub workflow_name: String,
    pub task: String,
    pub status: WorkflowRunStatus,
    pub current_step: Option<usize>,
    pub checkpoint: Option<CheckpointInfo>,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Run status
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunStatus {
    Running,
    AwaitingCheckpoint,
    Completed,
    Failed,
}

/// Checkpoint info for web UI
#[derive(Debug, Clone, Serialize)]
pub struct CheckpointInfo {
    pub message: String,
    pub content: Option<String>,
}

// ============================================================================
// Request types
// ============================================================================

/// Run workflow request
#[derive(Debug, Deserialize)]
pub struct RunWorkflowRequest {
    pub task: String,
    #[serde(default)]
    pub non_interactive: bool,
}

/// Checkpoint response request
#[derive(Debug, Deserialize)]
pub struct CheckpointResponseRequest {
    pub approved: bool,
    pub feedback: Option<String>,
}

// ============================================================================
// In-memory workflow run storage
// ============================================================================

/// Active workflow runs (in-memory for now)
pub type WorkflowRuns = Arc<RwLock<HashMap<String, WorkflowRunInfo>>>;

/// Create a new workflow runs store
pub fn new_workflow_runs() -> WorkflowRuns {
    Arc::new(RwLock::new(HashMap::new()))
}

// ============================================================================
// Handlers
// ============================================================================

/// List available workflows
/// GET /api/workflows
pub async fn list_workflows(
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkflowSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let config = EngineConfig {
        ollama_url: state.ollama_url.clone(),
        default_model: state.model.clone(),
        non_interactive: true,
        verbose: false,
        custom_workflows_dir: None,
    };
    let registry = AgentRegistry::with_defaults(&state.model);
    let engine = WorkflowEngine::new(registry, config);

    let workflows: Vec<WorkflowSummary> = engine
        .list_workflows()
        .into_iter()
        .map(|(name, description, is_custom)| WorkflowSummary {
            name: name.to_string(),
            description: description.to_string(),
            is_custom,
        })
        .collect();

    Ok(Json(workflows))
}

/// Get workflow details
/// GET /api/workflows/:name
pub async fn get_workflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<WorkflowDetail>, (StatusCode, Json<ErrorResponse>)> {
    let config = EngineConfig {
        ollama_url: state.ollama_url.clone(),
        default_model: state.model.clone(),
        non_interactive: true,
        verbose: false,
        custom_workflows_dir: None,
    };
    let registry = AgentRegistry::with_defaults(&state.model);
    let engine = WorkflowEngine::new(registry, config);

    match engine.get_workflow(&name) {
        Some(workflow) => {
            let steps: Vec<WorkflowStepInfo> = workflow
                .steps
                .iter()
                .map(|step| match step {
                    WorkflowStep::Agent { name, task, .. } => WorkflowStepInfo {
                        step_type: "agent".to_string(),
                        agent_name: Some(name.clone()),
                        task_template: Some(task.clone()),
                        checkpoint_message: None,
                    },
                    WorkflowStep::Checkpoint { message, .. } => WorkflowStepInfo {
                        step_type: "checkpoint".to_string(),
                        agent_name: None,
                        task_template: None,
                        checkpoint_message: Some(message.clone()),
                    },
                    _ => WorkflowStepInfo {
                        step_type: "other".to_string(),
                        agent_name: None,
                        task_template: None,
                        checkpoint_message: None,
                    },
                })
                .collect();

            Ok(Json(WorkflowDetail {
                name: workflow.name.clone(),
                description: workflow.description.clone(),
                steps,
            }))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!("Workflow '{}' not found", name))),
        )),
    }
}

/// List available agents
/// GET /api/agents
pub async fn list_agents(
    State(state): State<AppState>,
) -> Result<Json<Vec<AgentSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let registry = AgentRegistry::with_defaults(&state.model);

    let agents: Vec<AgentSummary> = registry
        .iter()
        .map(|(name, config)| AgentSummary {
            name: name.to_string(),
            display_name: config.display_name.clone(),
            model: config.model.clone(),
            temperature: config.temperature,
            tools: config.tools.clone(),
            can_handoff_to: config.can_handoff_to.clone(),
        })
        .collect();

    Ok(Json(agents))
}

/// Start a workflow run
/// POST /api/workflows/:name/run
pub async fn run_workflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<RunWorkflowRequest>,
) -> Result<(StatusCode, Json<WorkflowRunInfo>), (StatusCode, Json<ErrorResponse>)> {
    // Validate workflow exists
    let config = EngineConfig {
        ollama_url: state.ollama_url.clone(),
        default_model: state.model.clone(),
        non_interactive: req.non_interactive,
        verbose: false,
        custom_workflows_dir: None,
    };
    let registry = AgentRegistry::with_defaults(&state.model);
    let engine = WorkflowEngine::new(registry, config);

    if engine.get_workflow(&name).is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!("Workflow '{}' not found", name))),
        ));
    }

    // Create run record
    let run_id = Uuid::new_v4().to_string();
    let run_info = WorkflowRunInfo {
        id: run_id.clone(),
        workflow_name: name.clone(),
        task: req.task.clone(),
        status: WorkflowRunStatus::Running,
        current_step: Some(0),
        checkpoint: None,
        output: None,
        error: None,
    };

    // Store run (would be in state.workflow_runs in full implementation)
    // For now, we'll run synchronously in non-interactive mode
    // TODO: Implement async execution with checkpoint polling

    if req.non_interactive {
        // Run synchronously for non-interactive mode
        let result = engine.run(&name, &req.task).await;

        let final_run_info = match result {
            Ok(result) => {
                let output = result.context.get("changes")
                    .or_else(|| result.context.get("plan"))
                    .or_else(|| result.context.get("review"))
                    .cloned();

                WorkflowRunInfo {
                    id: run_id,
                    workflow_name: name,
                    task: req.task,
                    status: match result.status {
                        WorkflowStatus::Completed => WorkflowRunStatus::Completed,
                        WorkflowStatus::Failed { .. } => WorkflowRunStatus::Failed,
                        WorkflowStatus::Cancelled => WorkflowRunStatus::Failed,
                        WorkflowStatus::Running { .. } => WorkflowRunStatus::Running,
                    },
                    current_step: None,
                    checkpoint: None,
                    output,
                    error: None,
                }
            }
            Err(e) => WorkflowRunInfo {
                id: run_id,
                workflow_name: name,
                task: req.task,
                status: WorkflowRunStatus::Failed,
                current_step: None,
                checkpoint: None,
                output: None,
                error: Some(e.to_string()),
            },
        };

        Ok((StatusCode::OK, Json(final_run_info)))
    } else {
        // For interactive mode, return immediately with "running" status
        // The actual execution would happen in a background task
        // and the client would poll for status
        //
        // TODO: Implement background execution with checkpoint handling
        // For now, return an error indicating interactive mode needs more work
        Err((
            StatusCode::NOT_IMPLEMENTED,
            Json(ErrorResponse::new(
                "Interactive workflow runs via web API are not yet implemented. Use --non-interactive or the CLI."
            )),
        ))
    }
}

/// Get workflow run status
/// GET /api/workflows/runs/:id
pub async fn get_run_status(
    Path(run_id): Path<String>,
) -> Result<Json<WorkflowRunInfo>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Look up run from storage
    // For now, return not found since we don't persist runs yet
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(format!("Run '{}' not found", run_id))),
    ))
}

/// Submit checkpoint response
/// POST /api/workflows/runs/:id/checkpoint
pub async fn submit_checkpoint(
    Path(run_id): Path<String>,
    Json(_req): Json<CheckpointResponseRequest>,
) -> Result<Json<WorkflowRunInfo>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Look up run and submit checkpoint response
    // For now, return not found
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(format!("Run '{}' not found", run_id))),
    ))
}
