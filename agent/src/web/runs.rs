//! Runs API handlers
//!
//! Provides REST endpoints for workflow run analysis:
//! - List/view completed runs
//! - Get run events and metrics
//! - Export runs as markdown for analysis
//! - List and record improvements

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use super::state::AppState;
use crate::db::runs::{
    Improvement, ImprovementCategory, ImprovementFilter, Run, RunEvent, RunFilter, RunMetrics,
    RunStatus, RunSummary,
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

/// Run summary for listing
#[derive(Debug, Serialize)]
pub struct RunSummaryResponse {
    pub id: String,
    pub workflow_name: String,
    pub task: String,
    pub status: String,
    pub model: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
}

impl From<Run> for RunSummaryResponse {
    fn from(run: Run) -> Self {
        Self {
            id: run.id,
            workflow_name: run.workflow_name,
            task: run.task,
            status: run.status.to_string(),
            model: run.model,
            started_at: run.started_at.to_rfc3339(),
            completed_at: run.completed_at.map(|dt| dt.to_rfc3339()),
            duration_ms: run.duration_ms,
        }
    }
}

impl From<RunSummary> for RunSummaryResponse {
    fn from(run: RunSummary) -> Self {
        Self {
            id: run.id,
            workflow_name: run.workflow_name,
            task: run.task,
            status: run.status.to_string(),
            model: run.model,
            started_at: run.started_at.to_rfc3339(),
            completed_at: None, // RunSummary doesn't have completed_at
            duration_ms: run.duration_ms,
        }
    }
}

/// Runs list response
#[derive(Debug, Serialize)]
pub struct RunsListResponse {
    pub runs: Vec<RunSummaryResponse>,
    pub total: usize,
}

/// Run detail response (includes context)
#[derive(Debug, Serialize)]
pub struct RunDetailResponse {
    pub id: String,
    pub workflow_name: String,
    pub task: String,
    pub status: String,
    pub model: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub context: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl From<Run> for RunDetailResponse {
    fn from(run: Run) -> Self {
        Self {
            id: run.id,
            workflow_name: run.workflow_name,
            task: run.task,
            status: run.status.to_string(),
            model: run.model,
            started_at: run.started_at.to_rfc3339(),
            completed_at: run.completed_at.map(|dt| dt.to_rfc3339()),
            duration_ms: run.duration_ms,
            context: run.context,
            error: run.error,
        }
    }
}

/// Run events response
#[derive(Debug, Serialize)]
pub struct RunEventsResponse {
    pub run_id: String,
    pub events: Vec<RunEventResponse>,
}

/// Single run event
#[derive(Debug, Serialize)]
pub struct RunEventResponse {
    pub id: i64,
    pub step_index: usize,
    pub event_type: String,
    pub event_data: serde_json::Value,
    pub timestamp: String,
}

impl From<RunEvent> for RunEventResponse {
    fn from(event: RunEvent) -> Self {
        Self {
            id: event.id,
            step_index: event.step_index,
            event_type: event.event_type,
            event_data: event.event_data,
            timestamp: event.timestamp.to_rfc3339(),
        }
    }
}

/// Run metrics response
#[derive(Debug, Serialize)]
pub struct RunMetricsResponse {
    pub run_id: String,
    pub total_tool_calls: i64,
    pub successful_tool_calls: i64,
    pub failed_tool_calls: i64,
    pub total_tokens_in: Option<i64>,
    pub total_tokens_out: Option<i64>,
    pub files_read: i64,
    pub files_modified: i64,
}

impl RunMetricsResponse {
    fn from_metrics(run_id: String, metrics: RunMetrics) -> Self {
        Self {
            run_id,
            total_tool_calls: metrics.total_tool_calls as i64,
            successful_tool_calls: metrics.successful_tool_calls as i64,
            failed_tool_calls: metrics.failed_tool_calls as i64,
            total_tokens_in: metrics.total_tokens_in,
            total_tokens_out: metrics.total_tokens_out,
            files_read: metrics.files_read as i64,
            files_modified: metrics.files_modified as i64,
        }
    }
}

/// Export response (markdown)
#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub run_id: String,
    pub format: String,
    pub content: String,
}

/// Improvement response
#[derive(Debug, Serialize)]
pub struct ImprovementResponse {
    pub id: String,
    pub category: String,
    pub description: String,
    pub related_runs: Vec<String>,
    pub changes_made: Option<String>,
    pub impact: Option<String>,
    pub created_at: String,
}

impl From<Improvement> for ImprovementResponse {
    fn from(imp: Improvement) -> Self {
        Self {
            id: imp.id,
            category: imp.category.to_string(),
            description: imp.description,
            related_runs: imp.related_runs,
            changes_made: imp.changes_made,
            impact: imp.impact.map(|v| serde_json::to_string(&v).unwrap_or_default()),
            created_at: imp.created_at.to_rfc3339(),
        }
    }
}

/// Improvements list response
#[derive(Debug, Serialize)]
pub struct ImprovementsListResponse {
    pub improvements: Vec<ImprovementResponse>,
    pub total: usize,
}

// ============================================================================
// Query parameters
// ============================================================================

/// Runs filter query parameters
#[derive(Debug, Deserialize)]
pub struct RunsQueryParams {
    pub limit: Option<u32>,
    pub workflow: Option<String>,
    pub status: Option<String>,
}

impl From<RunsQueryParams> for RunFilter {
    fn from(params: RunsQueryParams) -> Self {
        let status = params.status.and_then(|s| match s.as_str() {
            "running" => Some(RunStatus::Running),
            "completed" => Some(RunStatus::Completed),
            "failed" => Some(RunStatus::Failed),
            "cancelled" => Some(RunStatus::Cancelled),
            _ => None,
        });

        RunFilter {
            limit: params.limit,
            workflow_name: params.workflow,
            status,
            offset: None,
        }
    }
}

/// Improvements filter query parameters
#[derive(Debug, Deserialize)]
pub struct ImprovementsQueryParams {
    pub limit: Option<u32>,
    pub category: Option<String>,
}

impl From<ImprovementsQueryParams> for ImprovementFilter {
    fn from(params: ImprovementsQueryParams) -> Self {
        let category = params.category.and_then(|s| match s.as_str() {
            "prompt" => Some(ImprovementCategory::Prompt),
            "workflow" => Some(ImprovementCategory::Workflow),
            "agent" => Some(ImprovementCategory::Agent),
            "tool" => Some(ImprovementCategory::Tool),
            "other" => Some(ImprovementCategory::Other),
            _ => None,
        });

        ImprovementFilter {
            limit: params.limit,
            category,
            status: None,
            offset: None,
        }
    }
}

// ============================================================================
// Request types
// ============================================================================

/// Create improvement request
#[derive(Debug, Deserialize)]
pub struct CreateImprovementRequest {
    pub category: String,
    pub description: String,
    pub related_runs: Option<Vec<String>>,
}

// ============================================================================
// Handlers
// ============================================================================

/// List runs with optional filters
/// GET /api/runs
pub async fn list_runs(
    State(state): State<AppState>,
    Query(params): Query<RunsQueryParams>,
) -> Result<Json<RunsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let filter: RunFilter = params.into();

    match state.db.list_runs(&filter) {
        Ok(runs) => {
            let total = runs.len();
            let runs: Vec<RunSummaryResponse> = runs.into_iter().map(Into::into).collect();
            Ok(Json(RunsListResponse { runs, total }))
        }
        Err(e) => {
            tracing::error!("Failed to list runs: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

/// Get a specific run
/// GET /api/runs/:id
pub async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RunDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.db.get_run(&id) {
        Ok(Some(run)) => Ok(Json(run.into())),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!("Run '{}' not found", id))),
        )),
        Err(e) => {
            tracing::error!("Failed to get run: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

/// Get events for a run
/// GET /api/runs/:id/events
pub async fn get_run_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RunEventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // First verify run exists
    match state.db.get_run(&id) {
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(format!("Run '{}' not found", id))),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to get run: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ));
        }
        Ok(Some(_)) => {}
    }

    match state.db.get_run_events(&id) {
        Ok(events) => {
            let events: Vec<RunEventResponse> = events.into_iter().map(Into::into).collect();
            Ok(Json(RunEventsResponse { run_id: id, events }))
        }
        Err(e) => {
            tracing::error!("Failed to get run events: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

/// Get metrics for a run
/// GET /api/runs/:id/metrics
pub async fn get_run_metrics(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RunMetricsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // First verify run exists
    match state.db.get_run(&id) {
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(format!("Run '{}' not found", id))),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to get run: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ));
        }
        Ok(Some(_)) => {}
    }

    match state.db.get_run_metrics(&id) {
        Ok(Some(metrics)) => Ok(Json(RunMetricsResponse::from_metrics(id, metrics))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(format!(
                "No metrics found for run '{}'",
                id
            ))),
        )),
        Err(e) => {
            tracing::error!("Failed to get run metrics: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

/// Export a run as markdown
/// GET /api/runs/:id/export
pub async fn export_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ExportResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get run
    let run = match state.db.get_run(&id) {
        Ok(Some(run)) => run,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(format!("Run '{}' not found", id))),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to get run: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ));
        }
    };

    // Get events
    let events = state.db.get_run_events(&id).unwrap_or_default();

    // Get metrics
    let metrics = state.db.get_run_metrics(&id).ok().flatten();

    // Generate markdown
    let content = export_markdown(&run, &events, metrics.as_ref());

    Ok(Json(ExportResponse {
        run_id: id,
        format: "markdown".to_string(),
        content,
    }))
}

/// List improvements with optional filters
/// GET /api/improvements
pub async fn list_improvements(
    State(state): State<AppState>,
    Query(params): Query<ImprovementsQueryParams>,
) -> Result<Json<ImprovementsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let filter: ImprovementFilter = params.into();

    match state.db.list_improvements(&filter) {
        Ok(improvements) => {
            let total = improvements.len();
            let improvements: Vec<ImprovementResponse> =
                improvements.into_iter().map(Into::into).collect();
            Ok(Json(ImprovementsListResponse {
                improvements,
                total,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to list improvements: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

/// Create a new improvement
/// POST /api/improvements
pub async fn create_improvement(
    State(state): State<AppState>,
    Json(req): Json<CreateImprovementRequest>,
) -> Result<(StatusCode, Json<ImprovementResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Parse category
    let category = match req.category.as_str() {
        "prompt" => ImprovementCategory::Prompt,
        "workflow" => ImprovementCategory::Workflow,
        "agent" => ImprovementCategory::Agent,
        "tool" => ImprovementCategory::Tool,
        "other" => ImprovementCategory::Other,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(format!(
                    "Invalid category: {}. Must be one of: prompt, workflow, agent, tool, other",
                    req.category
                ))),
            ))
        }
    };

    let related_runs = req.related_runs.unwrap_or_default();

    match state
        .db
        .create_improvement(category, &req.description, &related_runs)
    {
        Ok(improvement) => Ok((StatusCode::CREATED, Json(improvement.into()))),
        Err(e) => {
            tracing::error!("Failed to create improvement: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Format duration in milliseconds to human-readable string
fn format_duration(ms: i64) -> String {
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes % 60)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds % 60)
    } else if seconds > 0 {
        format!("{}s", seconds)
    } else {
        format!("{}ms", ms)
    }
}

/// Export a run as markdown
fn export_markdown(run: &Run, events: &[RunEvent], metrics: Option<&RunMetrics>) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&format!("# Run Analysis: {}\n\n", run.id));

    // Overview
    output.push_str("## Overview\n\n");
    output.push_str(&format!("- **Workflow:** {}\n", run.workflow_name));
    output.push_str(&format!("- **Task:** {}\n", run.task));
    output.push_str(&format!("- **Status:** {}\n", run.status));
    output.push_str(&format!("- **Model:** {}\n", run.model));
    if let Some(duration) = run.duration_ms {
        output.push_str(&format!("- **Duration:** {}\n", format_duration(duration)));
    }
    output.push_str(&format!("- **Started:** {}\n", run.started_at.to_rfc3339()));
    if let Some(ref completed) = run.completed_at {
        output.push_str(&format!("- **Completed:** {}\n", completed.to_rfc3339()));
    }
    output.push('\n');

    // Metrics
    if let Some(m) = metrics {
        output.push_str("## Metrics\n\n");
        output.push_str(&format!("- **Total Tool Calls:** {}\n", m.total_tool_calls));
        output.push_str(&format!("- **Successful:** {}\n", m.successful_tool_calls));
        output.push_str(&format!("- **Failed:** {}\n", m.failed_tool_calls));
        output.push_str(&format!("- **Files Read:** {}\n", m.files_read));
        output.push_str(&format!("- **Files Modified:** {}\n", m.files_modified));
        if let (Some(tokens_in), Some(tokens_out)) = (m.total_tokens_in, m.total_tokens_out) {
            output.push_str(&format!(
                "- **Tokens (in/out):** {} / {}\n",
                tokens_in, tokens_out
            ));
        }
        output.push('\n');
    }

    // Events summary
    if !events.is_empty() {
        output.push_str("## Events\n\n");

        // Group by step
        let mut current_step: Option<usize> = None;
        for event in events {
            if current_step != Some(event.step_index) {
                current_step = Some(event.step_index);
                output.push_str(&format!("### Step {}\n\n", event.step_index));
            }

            // Format event based on type
            match event.event_type.as_str() {
                "tool_start" => {
                    if let Some(name) = event.event_data.get("name") {
                        output.push_str(&format!("- **Tool Start:** {}\n", name));
                    }
                }
                "tool_complete" => {
                    if let Some(name) = event.event_data.get("name") {
                        let is_error = event
                            .event_data
                            .get("is_error")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let status = if is_error { "FAILED" } else { "OK" };
                        output.push_str(&format!("- **Tool Complete:** {} ({})\n", name, status));
                    }
                }
                "error" => {
                    if let Some(msg) = event.event_data.get("message") {
                        output.push_str(&format!("- **Error:** {}\n", msg));
                    }
                }
                _ => {
                    output.push_str(&format!(
                        "- **{}:** {}\n",
                        event.event_type,
                        event.timestamp.to_rfc3339()
                    ));
                }
            }
        }
        output.push('\n');
    }

    // Context
    if let Some(ref context) = run.context {
        let has_content = match context {
            serde_json::Value::Object(map) => !map.is_empty(),
            serde_json::Value::Array(arr) => !arr.is_empty(),
            serde_json::Value::Null => false,
            _ => true,
        };

        if has_content {
            output.push_str("## Context\n\n");
            output.push_str("```json\n");
            output.push_str(&serde_json::to_string_pretty(context).unwrap_or_default());
            output.push_str("\n```\n\n");
        }
    }

    // Error if any
    if let Some(ref error) = run.error {
        output.push_str("## Error\n\n");
        output.push_str(&format!("```\n{}\n```\n\n", error));
    }

    output
}
