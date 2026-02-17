//! Type definitions for task-mcp

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Error type for parsing TaskStatus from string
#[derive(Debug, Clone)]
pub struct ParseTaskStatusError(String);

impl fmt::Display for ParseTaskStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid task status: {}", self.0)
    }
}

impl std::error::Error for ParseTaskStatusError {}

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
}

impl FromStr for TaskStatus {
    type Err = ParseTaskStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(TaskStatus::Pending),
            "in_progress" => Ok(TaskStatus::InProgress),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            "blocked" => Ok(TaskStatus::Blocked),
            _ => Err(ParseTaskStatusError(s.to_string())),
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
#[allow(dead_code)]
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
