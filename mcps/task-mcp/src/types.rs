//! Type definitions for task-mcp

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Task status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Blocked => "blocked",
        }
    }

    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "pending" => Ok(TaskStatus::Pending),
            "in_progress" => Ok(TaskStatus::InProgress),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            "blocked" => Ok(TaskStatus::Blocked),
            _ => anyhow::bail!("Invalid task status: {}", s),
        }
    }
}

/// Task representation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: i32,
    pub plan_source: Option<String>,
    pub plan_section: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub assigned_to: Option<String>,
    pub branch_name: Option<String>,
    pub pr_url: Option<String>,
    pub parent_task_id: Option<String>,
    pub metadata: Option<String>,
}

/// Task dependency representation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskDependency {
    pub task_id: String,
    pub depends_on_task_id: String,
    pub created_at: String,
}

/// Task execution representation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskExecution {
    pub id: String,
    pub task_id: String,
    pub run_id: Option<String>,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
}

/// Response for task list operations
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskListResponse {
    pub tasks: Vec<Task>,
    pub total: usize,
}

/// Response for dependency list operations
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DependencyListResponse {
    pub dependencies: Vec<TaskDependency>,
    pub blocking_tasks: Vec<Task>,
    pub blocked_tasks: Vec<Task>,
}

/// Response for checking blocking tasks
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BlockingCheckResponse {
    pub is_blocked: bool,
    pub blocking_task_ids: Vec<String>,
    pub blocking_tasks: Vec<Task>,
}
