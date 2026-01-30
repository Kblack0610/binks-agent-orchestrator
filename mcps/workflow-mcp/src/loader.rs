//! Workflow loading utilities
//!
//! This module provides functions for loading built-in and custom workflows.

use std::collections::HashMap;
use std::path::Path;

use crate::types::{Workflow, WorkflowError, WorkflowStep};

/// Get all built-in workflows
///
/// Returns a map of workflow names to workflow definitions.
/// Built-in workflows include:
/// - implement-feature: Plan, implement, and review a new feature
/// - fix-bug: Investigate, fix, and test a bug
/// - refactor: Plan and execute a refactoring
/// - quick-fix: Quick fix without planning
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
            .with_step(WorkflowStep::agent(
                "implementer",
                "Implement based on plan:\n\n{plan}",
            ))
            .with_step(WorkflowStep::agent(
                "reviewer",
                "Review the changes:\n\n{changes}",
            )),
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
            .with_step(WorkflowStep::agent(
                "implementer",
                "Fix based on investigation:\n\n{investigation}",
            ))
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
            .with_step(WorkflowStep::agent(
                "implementer",
                "Execute refactoring:\n\n{plan}",
            ))
            .with_step(WorkflowStep::agent(
                "reviewer",
                "Review refactoring:\n\n{changes}",
            )),
    );

    // Quick Fix workflow (no checkpoint)
    workflows.insert(
        "quick-fix".to_string(),
        Workflow::new("quick-fix")
            .with_description("Quick fix without planning - for simple changes")
            .with_step(WorkflowStep::agent(
                "implementer",
                "Make this change: {task}",
            ))
            .with_step(WorkflowStep::agent(
                "tester",
                "Verify the change:\n\n{changes}",
            )),
    );

    workflows
}

/// Load custom workflows from a directory
///
/// Scans the given directory for .toml files and attempts to parse each
/// as a workflow definition. Invalid workflows are logged as warnings
/// and skipped.
///
/// # Arguments
///
/// * `dir` - Path to directory containing workflow TOML files
///
/// # Returns
///
/// A map of workflow names to workflow definitions
pub fn load_custom_workflows(dir: &Path) -> Result<HashMap<String, Workflow>, WorkflowError> {
    let mut workflows = HashMap::new();

    if !dir.exists() {
        return Ok(workflows);
    }

    let entries = std::fs::read_dir(dir).map_err(|e| WorkflowError::IoError(e.to_string()))?;

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
    fn test_builtin_workflows() {
        let workflows = builtin_workflows();

        assert_eq!(workflows.len(), 4);
        assert!(workflows.contains_key("implement-feature"));
        assert!(workflows.contains_key("fix-bug"));
        assert!(workflows.contains_key("refactor"));
        assert!(workflows.contains_key("quick-fix"));

        // Verify implement-feature workflow structure
        let impl_feature = workflows.get("implement-feature").unwrap();
        assert_eq!(impl_feature.steps.len(), 4);
    }
}
