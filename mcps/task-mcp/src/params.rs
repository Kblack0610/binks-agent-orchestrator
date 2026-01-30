//! Parameter definitions for task-mcp tools

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// CRUD Operations
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateTaskParams {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub plan_source: Option<String>,
    #[serde(default)]
    pub plan_section: Option<String>,
    #[serde(default)]
    pub assigned_to: Option<String>,
    #[serde(default)]
    pub parent_task_id: Option<String>,
    #[serde(default)]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetTaskParams {
    /// Task ID or prefix (minimum 8 characters)
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListTasksParams {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub plan_source: Option<String>,
    #[serde(default)]
    pub assigned_to: Option<String>,
    #[serde(default)]
    pub min_priority: Option<i32>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateTaskParams {
    pub id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub branch_name: Option<String>,
    #[serde(default)]
    pub pr_url: Option<String>,
    #[serde(default)]
    pub assigned_to: Option<String>,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub metadata: Option<String>,
}

// ============================================================================
// Dependency Management
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddDependencyParams {
    pub task_id: String,
    pub depends_on_task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListDependenciesParams {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CheckBlockingTasksParams {
    pub task_id: String,
}

// ============================================================================
// Execution Tracking
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecordExecutionParams {
    pub task_id: String,
    pub status: String,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub result_summary: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LinkToRunParams {
    pub task_id: String,
    pub run_id: String,
}

// ============================================================================
// Query Operations
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TasksByStatusParams {
    pub status: String,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TasksByPlanParams {
    pub plan_source: String,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GrabNextTaskParams {
    pub agent_name: String,
    #[serde(default)]
    pub status_filter: Option<String>,
}

// ============================================================================
// Memory Integration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyncToMemoryParams {
    pub task_id: String,
    #[serde(default)]
    pub include_dependencies: Option<bool>,
}
