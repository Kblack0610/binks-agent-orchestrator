//! MCP parameter types for workflow tools

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for execute_workflow tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteWorkflowParams {
    /// Workflow name or path to TOML file
    pub workflow: String,

    /// Initial task description
    pub task: String,

    /// Optional context variables
    #[serde(default)]
    pub context: std::collections::HashMap<String, String>,
}

/// Parameters for get_execution_status tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecutionStatusParams {
    /// Execution ID returned from execute_workflow
    pub execution_id: String,
}

/// Parameters for resume_from_checkpoint tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResumeCheckpointParams {
    /// Execution ID to resume
    pub execution_id: String,

    /// User decision (approve/reject)
    pub approved: bool,

    /// Optional user feedback
    #[serde(default)]
    pub feedback: Option<String>,
}

/// Parameters for list_workflows tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListWorkflowsParams {
    /// Optional path to custom workflows directory
    #[serde(default)]
    pub custom_dir: Option<String>,
}
