//! Workflow definitions and primitives
//!
//! Workflows define multi-agent execution flows with steps like:
//! - Agent execution
//! - Human checkpoints
//! - Parallel execution (future)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A single step in a workflow
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

/// A complete workflow definition
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
        let content = std::fs::read_to_string(path)
            .map_err(|e| WorkflowError::IoError(e.to_string()))?;
        Self::from_toml(&content)
    }

    /// Load workflow from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, WorkflowError> {
        toml::from_str(toml_str)
            .map_err(|e| WorkflowError::ParseError(e.to_string()))
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

/// Errors that can occur with workflows
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

/// Collection of built-in workflows
pub fn builtin_workflows() -> HashMap<String, Workflow> {
    let mut workflows = HashMap::new();

    // Implement Feature workflow
    workflows.insert(
        "implement-feature".to_string(),
        Workflow::new("implement-feature")
            .with_description("Plan, implement, and review a new feature")
            .with_step(WorkflowStep::agent("planner", "Analyze and plan: {task}"))
            .with_step(WorkflowStep::Checkpoint {
                message: "Review the plan above. Proceed with implementation?".to_string(),
                show: Some("plan".to_string()),
            })
            .with_step(WorkflowStep::agent("implementer", "Implement based on plan:\n\n{plan}"))
            .with_step(WorkflowStep::agent("reviewer", "Review the changes:\n\n{changes}")),
    );

    // Fix Bug workflow
    workflows.insert(
        "fix-bug".to_string(),
        Workflow::new("fix-bug")
            .with_description("Investigate, fix, and test a bug")
            .with_step(WorkflowStep::agent("investigator", "Investigate: {task}"))
            .with_step(WorkflowStep::Checkpoint {
                message: "Review the investigation. Proceed with fix?".to_string(),
                show: Some("investigation".to_string()),
            })
            .with_step(WorkflowStep::agent("implementer", "Fix based on investigation:\n\n{investigation}"))
            .with_step(WorkflowStep::agent("tester", "Test the fix:\n\n{changes}")),
    );

    // Refactor workflow
    workflows.insert(
        "refactor".to_string(),
        Workflow::new("refactor")
            .with_description("Plan and execute a refactoring")
            .with_step(WorkflowStep::agent("planner", "Plan refactoring: {task}"))
            .with_step(WorkflowStep::Checkpoint {
                message: "Review the refactoring plan. Proceed?".to_string(),
                show: Some("plan".to_string()),
            })
            .with_step(WorkflowStep::agent("implementer", "Execute refactoring:\n\n{plan}"))
            .with_step(WorkflowStep::agent("reviewer", "Review refactoring:\n\n{changes}")),
    );

    // Quick Fix workflow (no checkpoint)
    workflows.insert(
        "quick-fix".to_string(),
        Workflow::new("quick-fix")
            .with_description("Quick fix without planning - for simple changes")
            .with_step(WorkflowStep::agent("implementer", "Make this change: {task}"))
            .with_step(WorkflowStep::agent("tester", "Verify the change:\n\n{changes}")),
    );

    workflows
}

/// Load custom workflows from a directory
pub fn load_custom_workflows(dir: &Path) -> Result<HashMap<String, Workflow>, WorkflowError> {
    let mut workflows = HashMap::new();

    if !dir.exists() {
        return Ok(workflows);
    }

    let entries = std::fs::read_dir(dir)
        .map_err(|e| WorkflowError::IoError(e.to_string()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match Workflow::from_toml_file(&path) {
                Ok(workflow) => {
                    workflows.insert(workflow.name.clone(), workflow);
                }
                Err(e) => {
                    tracing::warn!("Failed to load workflow from {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(workflows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_builder() {
        let workflow = Workflow::new("test")
            .with_description("A test workflow")
            .with_step(WorkflowStep::agent("planner", "{task}"))
            .with_step(WorkflowStep::checkpoint("Continue?"));

        assert_eq!(workflow.name, "test");
        assert_eq!(workflow.steps.len(), 2);
    }

    #[test]
    fn test_workflow_from_toml() {
        let toml = r#"
            name = "test-workflow"
            description = "A test workflow"

            [[steps]]
            type = "agent"
            name = "planner"
            task = "Plan: {task}"

            [[steps]]
            type = "checkpoint"
            message = "Approve plan?"
        "#;

        let workflow = Workflow::from_toml(toml).unwrap();
        assert_eq!(workflow.name, "test-workflow");
        assert_eq!(workflow.steps.len(), 2);
    }

    #[test]
    fn test_builtin_workflows() {
        let workflows = builtin_workflows();

        assert!(workflows.contains_key("implement-feature"));
        assert!(workflows.contains_key("fix-bug"));
        assert!(workflows.contains_key("refactor"));
        assert!(workflows.contains_key("quick-fix"));

        let feature = &workflows["implement-feature"];
        assert_eq!(feature.steps.len(), 4);
    }
}
