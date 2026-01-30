//! Workflow execution engine
//!
//! This module will contain:
//! - WorkflowExecutor - Executes workflow steps sequentially
//! - ExecutionState - Tracks execution progress
//! - ExecutionRegistry - Manages active executions
//! - Checkpoint handling logic
//! - Context variable interpolation

use crate::types::{WorkflowError, WorkflowResult, WorkflowStatus};
use std::collections::HashMap;
use uuid::Uuid;

/// State of a workflow execution
#[derive(Debug, Clone)]
pub struct ExecutionState {
    /// Unique execution ID
    pub id: String,

    /// Name of the workflow being executed
    pub workflow_name: String,

    /// Current step index
    pub current_step: usize,

    /// Execution status
    pub status: WorkflowStatus,

    /// Context variables accumulated during execution
    pub context: HashMap<String, String>,

    /// Results from completed steps
    pub step_results: Vec<crate::types::StepResult>,
}

impl ExecutionState {
    /// Create a new execution state
    pub fn new(workflow_name: impl Into<String>, initial_context: HashMap<String, String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            workflow_name: workflow_name.into(),
            current_step: 0,
            status: WorkflowStatus::Running { current_step: 0 },
            context: initial_context,
            step_results: Vec::new(),
        }
    }

    /// Convert to WorkflowResult
    pub fn to_result(self) -> WorkflowResult {
        WorkflowResult {
            workflow_name: self.workflow_name,
            step_results: self.step_results,
            status: self.status,
            context: self.context,
        }
    }
}

/// Registry for managing active workflow executions
#[derive(Debug, Default)]
pub struct ExecutionRegistry {
    executions: HashMap<String, ExecutionState>,
}

impl ExecutionRegistry {
    /// Create a new execution registry
    pub fn new() -> Self {
        Self {
            executions: HashMap::new(),
        }
    }

    /// Register a new execution
    pub fn register(&mut self, state: ExecutionState) -> String {
        let id = state.id.clone();
        self.executions.insert(id.clone(), state);
        id
    }

    /// Get execution state
    pub fn get(&self, id: &str) -> Result<&ExecutionState, WorkflowError> {
        self.executions
            .get(id)
            .ok_or_else(|| WorkflowError::ExecutionError(format!("Execution not found: {}", id)))
    }

    /// Get mutable execution state
    pub fn get_mut(&mut self, id: &str) -> Result<&mut ExecutionState, WorkflowError> {
        self.executions
            .get_mut(id)
            .ok_or_else(|| WorkflowError::ExecutionError(format!("Execution not found: {}", id)))
    }

    /// Remove execution
    pub fn remove(&mut self, id: &str) -> Option<ExecutionState> {
        self.executions.remove(id)
    }
}
