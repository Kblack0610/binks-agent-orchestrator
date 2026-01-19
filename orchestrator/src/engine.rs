//! Workflow execution engine
//!
//! Executes multi-agent workflows with:
//! - Sequential step execution
//! - Human-in-loop checkpoints
//! - Context passing between agents
//! - Per-agent model configuration

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;

use agent::agent::Agent;
use agent::config::{AgentFileConfig, McpConfig};
use agent::mcp::McpClientPool;

use crate::agent_config::AgentRegistry;
use crate::checkpoint::{Checkpoint, CheckpointHandler, CheckpointResult, InteractiveCheckpointHandler};
use crate::workflow::{
    builtin_workflows, load_custom_workflows, StepResult, Workflow, WorkflowError, WorkflowResult,
    WorkflowStatus, WorkflowStep,
};

/// Configuration for the workflow engine
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Ollama URL
    pub ollama_url: String,

    /// Default model (used if agent doesn't specify one)
    pub default_model: String,

    /// Directory for custom workflows
    pub custom_workflows_dir: Option<PathBuf>,

    /// Whether to run in non-interactive mode (auto-approve checkpoints)
    pub non_interactive: bool,

    /// Enable verbose output
    pub verbose: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            ollama_url: AgentFileConfig::default_ollama_url(),
            default_model: AgentFileConfig::default_model(),
            custom_workflows_dir: None,
            non_interactive: false,
            verbose: false,
        }
    }
}

impl EngineConfig {
    /// Create from agent file config
    pub fn from_agent_config(config: &AgentFileConfig) -> Self {
        Self {
            ollama_url: config.llm.url.clone(),
            default_model: config.llm.model.clone(),
            ..Default::default()
        }
    }
}

/// Workflow execution engine
pub struct WorkflowEngine {
    /// Agent registry with configurations
    registry: AgentRegistry,

    /// Engine configuration
    config: EngineConfig,

    /// Built-in workflows
    builtin_workflows: HashMap<String, Workflow>,

    /// Custom workflows loaded from files
    custom_workflows: HashMap<String, Workflow>,

    /// Checkpoint handler
    checkpoint_handler: Box<dyn CheckpointHandler>,
}

impl WorkflowEngine {
    /// Create a new workflow engine
    pub fn new(registry: AgentRegistry, config: EngineConfig) -> Self {
        let checkpoint_handler: Box<dyn CheckpointHandler> = if config.non_interactive {
            Box::new(crate::checkpoint::AutoApproveCheckpointHandler)
        } else {
            Box::new(InteractiveCheckpointHandler)
        };

        let mut custom_workflows = HashMap::new();
        if let Some(ref dir) = config.custom_workflows_dir {
            match load_custom_workflows(dir) {
                Ok(workflows) => custom_workflows = workflows,
                Err(e) => tracing::warn!("Failed to load custom workflows: {}", e),
            }
        }

        // Also try standard locations
        if let Some(config_dir) = dirs::config_dir() {
            let binks_workflows = config_dir.join("binks").join("workflows");
            if binks_workflows.exists() {
                if let Ok(workflows) = load_custom_workflows(&binks_workflows) {
                    custom_workflows.extend(workflows);
                }
            }
        }

        // Check local .binks/workflows
        if let Ok(cwd) = std::env::current_dir() {
            let local_workflows = cwd.join(".binks").join("workflows");
            if local_workflows.exists() {
                if let Ok(workflows) = load_custom_workflows(&local_workflows) {
                    custom_workflows.extend(workflows);
                }
            }
        }

        Self {
            registry,
            config,
            builtin_workflows: builtin_workflows(),
            custom_workflows,
            checkpoint_handler,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Result<Self> {
        let agent_config = AgentFileConfig::load()?;
        let config = EngineConfig::from_agent_config(&agent_config);
        let registry = AgentRegistry::with_defaults(&config.default_model);

        Ok(Self::new(registry, config))
    }

    /// Set a custom checkpoint handler
    pub fn with_checkpoint_handler(mut self, handler: Box<dyn CheckpointHandler>) -> Self {
        self.checkpoint_handler = handler;
        self
    }

    /// Get a workflow by name (checks custom first, then built-in)
    pub fn get_workflow(&self, name: &str) -> Option<&Workflow> {
        self.custom_workflows
            .get(name)
            .or_else(|| self.builtin_workflows.get(name))
    }

    /// List all available workflow names
    pub fn list_workflows(&self) -> Vec<(&str, &str, bool)> {
        let mut workflows: Vec<_> = self
            .builtin_workflows
            .iter()
            .map(|(name, w)| (name.as_str(), w.description.as_str(), false))
            .collect();

        workflows.extend(
            self.custom_workflows
                .iter()
                .map(|(name, w)| (name.as_str(), w.description.as_str(), true)),
        );

        workflows.sort_by_key(|(name, _, _)| *name);
        workflows
    }

    /// Run a workflow by name
    pub async fn run(&self, workflow_name: &str, task: &str) -> Result<WorkflowResult> {
        let workflow = self
            .get_workflow(workflow_name)
            .ok_or_else(|| WorkflowError::NotFound(workflow_name.to_string()))?
            .clone();

        self.execute(&workflow, task).await
    }

    /// Execute a workflow
    pub async fn execute(&self, workflow: &Workflow, task: &str) -> Result<WorkflowResult> {
        println!("\n{}", "═".repeat(60));
        println!("  WORKFLOW: {}", workflow.name);
        if !workflow.description.is_empty() {
            println!("  {}", workflow.description);
        }
        println!("{}\n", "═".repeat(60));

        // Load MCP config
        let mcp_config = McpConfig::load()?.ok_or_else(|| {
            anyhow::anyhow!("No .mcp.json found. MCP configuration is required for workflows.")
        })?;

        // Context for variable substitution
        let mut context: HashMap<String, String> = HashMap::new();
        context.insert("task".to_string(), task.to_string());

        let mut step_results = Vec::new();

        for (step_index, step) in workflow.steps.iter().enumerate() {
            let step_start = Instant::now();

            println!(
                "\n[Step {}/{}]",
                step_index + 1,
                workflow.steps.len()
            );

            match step {
                WorkflowStep::Agent {
                    name,
                    task: task_template,
                    model: model_override,
                } => {
                    // Get agent config
                    let agent_config = self.registry.get(name).ok_or_else(|| {
                        WorkflowError::AgentNotFound(name.clone())
                    })?;

                    // Determine model to use
                    let model = model_override
                        .as_ref()
                        .unwrap_or(&agent_config.model);

                    println!("  Agent: {} ({})", agent_config.display_name, model);

                    // Substitute variables in task
                    let task_text = self.substitute_variables(task_template, &context);

                    // Create MCP pool for this agent
                    let mcp_pool = McpClientPool::new(mcp_config.clone());

                    // Create agent with config
                    let mut agent = Agent::new(&self.config.ollama_url, model, mcp_pool)
                        .with_system_prompt(&agent_config.system_prompt)
                        .with_verbose(self.config.verbose);

                    // Run agent with tool filtering if specified
                    let output = if agent_config.tools.is_empty() {
                        agent.chat(&task_text).await?
                    } else {
                        let servers: Vec<&str> = agent_config.tools.iter().map(|s| s.as_str()).collect();
                        agent.chat_with_servers(&task_text, &servers).await?
                    };

                    // Store output in context
                    // Use agent name as context key (e.g., "plan" from planner, "changes" from implementer)
                    let context_key = match name.as_str() {
                        "planner" => "plan",
                        "investigator" => "investigation",
                        "implementer" => "changes",
                        "reviewer" => "review",
                        "tester" => "test_results",
                        other => other,
                    };
                    context.insert(context_key.to_string(), output.clone());

                    let duration_ms = step_start.elapsed().as_millis() as u64;
                    step_results.push(StepResult {
                        step_index,
                        output: output.clone(),
                        success: true,
                        duration_ms,
                    });

                    // Print output
                    println!("\n{}", "─".repeat(60));
                    println!("{}", output);
                    println!("{}{}", "─".repeat(60), "─".repeat(60));
                    println!("  Completed in {}ms", duration_ms);
                }

                WorkflowStep::Checkpoint { message, show } => {
                    println!("  Checkpoint: {}", message);

                    // Build checkpoint with optional content
                    let mut checkpoint = Checkpoint::new(message);

                    if let Some(key) = show {
                        if let Some(content) = context.get(key) {
                            checkpoint = checkpoint.with_content(content);
                        }
                    }

                    // Handle checkpoint
                    let result = self.checkpoint_handler.handle(&checkpoint);

                    match result {
                        CheckpointResult::Approved => {
                            println!("  ✓ Approved");
                        }
                        CheckpointResult::ApprovedWithNote(note) => {
                            println!("  ✓ Approved with note: {}", note);
                            context.insert("checkpoint_note".to_string(), note);
                        }
                        CheckpointResult::Rejected => {
                            println!("  ✗ Rejected - stopping workflow");
                            return Ok(WorkflowResult {
                                workflow_name: workflow.name.clone(),
                                step_results,
                                status: WorkflowStatus::Cancelled,
                                context,
                            });
                        }
                        CheckpointResult::Edit(edits) => {
                            println!("  ✎ Modifications provided");
                            context.insert("checkpoint_edits".to_string(), edits);
                        }
                    }

                    let duration_ms = step_start.elapsed().as_millis() as u64;
                    step_results.push(StepResult {
                        step_index,
                        output: "Checkpoint passed".to_string(),
                        success: true,
                        duration_ms,
                    });
                }

                WorkflowStep::Parallel(_steps) => {
                    // TODO: Implement parallel execution
                    println!("  ⚠ Parallel execution not yet implemented");
                    step_results.push(StepResult {
                        step_index,
                        output: "Parallel not implemented".to_string(),
                        success: false,
                        duration_ms: 0,
                    });
                }

                WorkflowStep::Branch { .. } => {
                    // TODO: Implement conditional branching
                    println!("  ⚠ Conditional branching not yet implemented");
                    step_results.push(StepResult {
                        step_index,
                        output: "Branch not implemented".to_string(),
                        success: false,
                        duration_ms: 0,
                    });
                }
            }
        }

        println!("\n{}", "═".repeat(60));
        println!("  WORKFLOW COMPLETED: {}", workflow.name);
        println!("{}\n", "═".repeat(60));

        Ok(WorkflowResult {
            workflow_name: workflow.name.clone(),
            step_results,
            status: WorkflowStatus::Completed,
            context,
        })
    }

    /// Substitute {variable} placeholders in a string
    fn substitute_variables(&self, template: &str, context: &HashMap<String, String>) -> String {
        println!("DEBUG: Substituting variables in template: {}", template);
        let mut result = template.to_string();

        for (key, value) in context {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
            println!("DEBUG: Replaced placeholder '{}' with value '{}'", placeholder, value);
        }

        println!("DEBUG: Final result after substitution: {}", result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_substitution() {
        let engine = WorkflowEngine {
            registry: AgentRegistry::new(),
            config: EngineConfig::default(),
            builtin_workflows: HashMap::new(),
            custom_workflows: HashMap::new(),
            checkpoint_handler: Box::new(crate::checkpoint::AutoApproveCheckpointHandler),
        };

        let mut context = HashMap::new();
        context.insert("task".to_string(), "Add dark mode".to_string());
        context.insert("plan".to_string(), "Step 1: ...".to_string());

        let result = engine.substitute_variables(
            "Implement based on plan:\n\n{plan}\n\nOriginal task: {task}",
            &context,
        );

        assert!(result.contains("Step 1: ..."));
        assert!(result.contains("Add dark mode"));
    }

    #[test]
    fn test_list_workflows() {
        let registry = AgentRegistry::with_defaults("test-model");
        let config = EngineConfig::default();
        let engine = WorkflowEngine::new(registry, config);

        let workflows = engine.list_workflows();
        let names: Vec<_> = workflows.iter().map(|(n, _, _)| *n).collect();

        assert!(names.contains(&"implement-feature"));
        assert!(names.contains(&"fix-bug"));
        assert!(names.contains(&"refactor"));
    }
}