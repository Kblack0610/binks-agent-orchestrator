//! Workflow and run parameter types

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Include all workflows (including disabled)")]
    pub all: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowRunParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Workflow ID or filename")]
    pub workflow: String,
    #[schemars(description = "Git ref (branch/tag) to run on")]
    pub ref_name: Option<String>,
    #[schemars(description = "JSON object of workflow inputs")]
    pub inputs: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Filter by workflow ID or filename")]
    pub workflow: Option<String>,
    #[schemars(description = "Filter by branch")]
    pub branch: Option<String>,
    #[schemars(description = "Filter by status (queued, in_progress, completed)")]
    pub status: Option<String>,
    #[schemars(description = "Maximum number of runs to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunViewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Workflow run ID")]
    pub run_id: u64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunCancelParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Workflow run ID")]
    pub run_id: u64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunLogParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Workflow run ID")]
    pub run_id: u64,
    #[schemars(description = "View logs for a specific job ID only")]
    pub job: Option<String>,
    #[schemars(description = "Only show logs from failed steps (default: false shows all logs)")]
    pub failed_only: Option<bool>,
    #[schemars(description = "The attempt number of the workflow run")]
    pub attempt: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunRerunParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Workflow run ID")]
    pub run_id: u64,
    #[schemars(description = "Only rerun failed jobs")]
    pub failed: Option<bool>,
    #[schemars(description = "Enable debug logging")]
    pub debug: Option<bool>,
}
