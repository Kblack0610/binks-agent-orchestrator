//! Core workflow type definitions
//!
//! This module contains all the type definitions for workflows, steps,
//! execution results, and errors.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A step in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowStep {
    /// Execute an agent with a task
    Agent {
        /// Name of the agent to run (must exist in registry)
        name: String,
        /// Task template (can use {task}, {plan}, {changes} placeholders)
        task: String,
        /// Optional: override the model for this step
        #[serde(default)]
        model: Option<String>,
    },

    /// Pause for human approval
    Checkpoint {
        /// Message to display to the user
        message: String,
        /// What to show the user (e.g., "plan", "changes")
        #[serde(default)]
        show: Option<String>,
    },

    /// Execute steps in parallel (future)
    #[serde(skip)]
    Parallel(Vec<WorkflowStep>),

    /// Conditional branching (future)
    #[serde(skip)]
    Branch {
        condition: String,
        on_true: Box<WorkflowStep>,
        on_false: Box<WorkflowStep>,
    },
}

impl WorkflowStep {
    /// Create an agent step
    pub fn agent(name: impl Into<String>, task: impl Into<String>) -> Self {
        Self::Agent {
            name: name.into(),
            task: task.into(),
            model: None,
        }
    }

    /// Create a checkpoint step
    pub fn checkpoint(message: impl Into<String>) -> Self {
        Self::Checkpoint {
            message: message.into(),
            show: None,
        }
    }

    /// Create a parallel step
    pub fn parallel(steps: Vec<WorkflowStep>) -> Self {
        Self::Parallel(steps)
    }
}

/// A workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique identifier for this workflow
    pub name: String,

    /// Human-readable description
    #[serde(default)]
    pub description: String,

    /// Workflow steps to execute
    pub steps: Vec<WorkflowStep>,
}

impl Workflow {
    /// Create a new workflow
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            steps: Vec::new(),
        }
    }

    /// Set the description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a step
    pub fn with_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Load workflow from TOML file
    pub fn from_toml_file(path: &Path) -> Result<Self, WorkflowError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| WorkflowError::IoError(e.to_string()))?;
        Self::from_toml(&content)
    }

    /// Load workflow from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, WorkflowError> {
        toml::from_str(toml_str).map_err(|e| WorkflowError::ParseError(e.to_string()))
    }
}

/// Result of executing a workflow step
#[derive(Debug, Clone)]
pub struct StepResult {
    /// The step that was executed
    pub step_index: usize,

    /// Output from the step
    pub output: String,

    /// Whether the step succeeded
    pub success: bool,

    /// Duration of execution
    pub duration_ms: u64,
}

/// Result of executing a complete workflow
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    /// Name of the workflow that was executed
    pub workflow_name: String,

    /// Results from each step
    pub step_results: Vec<StepResult>,

    /// Final status
    pub status: WorkflowStatus,

    /// Context variables accumulated during execution
    pub context: HashMap<String, String>,
}

/// Status of a workflow execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowStatus {
    /// Workflow completed successfully
    Completed,
    /// Workflow was cancelled by user at a checkpoint
    Cancelled,
    /// Workflow failed at a step
    Failed { step_index: usize, error: String },
    /// Workflow is still running
    Running { current_step: usize },
}

/// Workflow-related errors
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Workflow not found: {0}")]
    NotFound(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Checkpoint rejected by user")]
    CheckpointRejected,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_builder() {
        let workflow = Workflow::new("test")
            .with_description("Test workflow")
            .with_step(WorkflowStep::agent("planner", "Plan the task"))
            .with_step(WorkflowStep::checkpoint("Review the plan"));

        assert_eq!(workflow.name, "test");
        assert_eq!(workflow.description, "Test workflow");
        assert_eq!(workflow.steps.len(), 2);
    }

    #[test]
    fn test_workflow_from_toml() {
        let toml = r#"
            name = "test"
            description = "Test workflow"

            [[steps]]
            type = "agent"
            name = "planner"
            task = "Plan the task"

            [[steps]]
            type = "checkpoint"
            message = "Review the plan"
        "#;

        let workflow = Workflow::from_toml(toml).unwrap();
        assert_eq!(workflow.name, "test");
        assert_eq!(workflow.steps.len(), 2);
    }
}
