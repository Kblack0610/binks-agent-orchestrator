//! Mode state machine for interactive CLI
//!
//! This module defines the different modes the CLI can operate in,
//! each with its own prompt prefix and available commands.

use serde::{Deserialize, Serialize};

/// CLI mode state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// Normal conversation mode
    Normal,

    /// Planning mode - focus on analysis and planning
    Plan {
        /// Context for the plan
        context: String,
        /// Accumulated plan steps
        steps: Vec<String>,
    },

    /// Implementation mode - focus on code changes
    Implement {
        /// Reference to a plan (if any)
        plan: Option<String>,
        /// Files modified during implementation
        files_modified: Vec<String>,
    },
}

impl Default for Mode {
    fn default() -> Self {
        Self::Normal
    }
}

impl Mode {
    /// Get the prompt prefix for this mode
    pub fn prompt_prefix(&self) -> &'static str {
        match self {
            Mode::Normal => "agent",
            Mode::Plan { .. } => "plan",
            Mode::Implement { .. } => "impl",
        }
    }

    /// Get the mode name
    pub fn name(&self) -> &'static str {
        match self {
            Mode::Normal => "normal",
            Mode::Plan { .. } => "plan",
            Mode::Implement { .. } => "implement",
        }
    }

    /// Get a system prompt modifier for this mode
    pub fn system_prompt_modifier(&self) -> Option<String> {
        match self {
            Mode::Normal => None,
            Mode::Plan { context, .. } => Some(format!(
                "\n\n[MODE: PLANNING]\nFocus on analysis, architecture, and planning. \
                 Explain your thinking step by step. Do not write implementation code yet.\n\
                 Context: {}",
                context
            )),
            Mode::Implement { plan, .. } => {
                let plan_context = plan
                    .as_ref()
                    .map(|p| format!("\nPlan reference: {}", p))
                    .unwrap_or_default();
                Some(format!(
                    "\n\n[MODE: IMPLEMENTATION]\nFocus on writing clean, working code. \
                     Follow the plan if provided. Explain code changes briefly.{}",
                    plan_context
                ))
            }
        }
    }

    /// Check if this is normal mode
    pub fn is_normal(&self) -> bool {
        matches!(self, Mode::Normal)
    }

    /// Check if this is plan mode
    pub fn is_plan(&self) -> bool {
        matches!(self, Mode::Plan { .. })
    }

    /// Check if this is implement mode
    pub fn is_implement(&self) -> bool {
        matches!(self, Mode::Implement { .. })
    }

    /// Add a step to plan mode
    pub fn add_plan_step(&mut self, step: String) {
        if let Mode::Plan { steps, .. } = self {
            steps.push(step);
        }
    }

    /// Add a file to implement mode's modified list
    pub fn add_modified_file(&mut self, file: String) {
        if let Mode::Implement { files_modified, .. } = self {
            if !files_modified.contains(&file) {
                files_modified.push(file);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode() {
        let mode = Mode::default();
        assert!(mode.is_normal());
        assert_eq!(mode.prompt_prefix(), "agent");
    }

    #[test]
    fn test_plan_mode() {
        let mode = Mode::Plan {
            context: "Test context".to_string(),
            steps: vec![],
        };
        assert!(mode.is_plan());
        assert_eq!(mode.prompt_prefix(), "plan");
        assert!(mode.system_prompt_modifier().is_some());
    }

    #[test]
    fn test_implement_mode() {
        let mode = Mode::Implement {
            plan: Some("My plan".to_string()),
            files_modified: vec![],
        };
        assert!(mode.is_implement());
        assert_eq!(mode.prompt_prefix(), "impl");
    }

    #[test]
    fn test_add_plan_step() {
        let mut mode = Mode::Plan {
            context: "Test".to_string(),
            steps: vec![],
        };
        mode.add_plan_step("Step 1".to_string());

        if let Mode::Plan { steps, .. } = &mode {
            assert_eq!(steps.len(), 1);
            assert_eq!(steps[0], "Step 1");
        } else {
            panic!("Expected Plan mode");
        }
    }

    #[test]
    fn test_add_modified_file() {
        let mut mode = Mode::Implement {
            plan: None,
            files_modified: vec![],
        };
        mode.add_modified_file("file.rs".to_string());
        mode.add_modified_file("file.rs".to_string()); // Duplicate

        if let Mode::Implement { files_modified, .. } = &mode {
            assert_eq!(files_modified.len(), 1); // Should dedupe
        } else {
            panic!("Expected Implement mode");
        }
    }
}
