//! Workflow and workflow run type definitions
//!
//! Structs representing GitHub Actions workflows and runs as returned by gh CLI.

use serde::{Deserialize, Serialize};

/// Represents a GitHub Actions workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow name
    pub name: String,

    /// Workflow ID
    pub id: u64,

    /// Workflow file path (e.g., ".github/workflows/ci.yml")
    pub path: String,

    /// Workflow state (active, disabled, etc.)
    pub state: String,
}

/// Represents a GitHub Actions workflow run
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRun {
    /// Run number (sequential within workflow)
    #[serde(alias = "number")]
    pub run_number: u32,

    /// Database ID for the run
    pub database_id: u64,

    /// Run status (queued, in_progress, completed)
    pub status: String,

    /// Run conclusion (success, failure, cancelled, etc.)
    #[serde(default)]
    pub conclusion: Option<String>,

    /// Workflow name
    pub name: String,

    /// Event that triggered the run (push, pull_request, etc.)
    pub event: String,

    /// Branch name
    pub head_branch: String,

    /// Commit SHA
    pub head_sha: String,

    /// Run URL on GitHub
    pub url: String,

    /// Creation timestamp (ISO 8601)
    pub created_at: String,

    /// Update timestamp (ISO 8601)
    pub updated_at: String,

    /// Start time (ISO 8601)
    #[serde(default)]
    pub run_started_at: Option<String>,

    /// Display title
    #[serde(default)]
    pub display_title: Option<String>,
}

/// Represents a job within a workflow run
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowJob {
    /// Job ID
    pub id: u64,

    /// Job name
    pub name: String,

    /// Job status
    pub status: String,

    /// Job conclusion
    #[serde(default)]
    pub conclusion: Option<String>,

    /// Start time
    #[serde(default)]
    pub started_at: Option<String>,

    /// Completion time
    #[serde(default)]
    pub completed_at: Option<String>,

    /// Job steps
    #[serde(default)]
    pub steps: Vec<WorkflowStep>,
}

/// Represents a step within a workflow job
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step name
    pub name: String,

    /// Step status
    pub status: String,

    /// Step conclusion
    #[serde(default)]
    pub conclusion: Option<String>,

    /// Step number
    pub number: u32,
}

impl Workflow {
    /// Returns the JSON fields for workflow list
    pub fn list_fields() -> &'static [&'static str] {
        &["name", "id", "path", "state"]
    }
}

impl WorkflowRun {
    /// Returns the JSON fields for run list
    pub fn list_fields() -> &'static [&'static str] {
        &[
            "number",
            "databaseId",
            "status",
            "conclusion",
            "name",
            "event",
            "headBranch",
            "headSha",
            "url",
            "createdAt",
            "updatedAt",
        ]
    }

    /// Returns the JSON fields for run view
    pub fn view_fields() -> &'static [&'static str] {
        &[
            "number",
            "databaseId",
            "status",
            "conclusion",
            "name",
            "event",
            "headBranch",
            "headSha",
            "url",
            "createdAt",
            "updatedAt",
            "runStartedAt",
            "displayTitle",
        ]
    }
}
