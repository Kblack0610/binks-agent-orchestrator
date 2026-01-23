//! Workflow command handler
//!
//! Run multi-agent workflows.

use anyhow::Result;

use super::CommandContext;
use crate::cli::WorkflowCommands;
use crate::orchestrator::workflow::WorkflowStep;
use crate::orchestrator::{AgentRegistry, EngineConfig, WorkflowEngine};

/// Handle the `workflow` command
pub async fn run_workflow_command(ctx: &CommandContext, command: WorkflowCommands) -> Result<()> {
    match command {
        WorkflowCommands::List => run_list(ctx).await,
        WorkflowCommands::Show { name } => run_show(ctx, &name).await,
        WorkflowCommands::Run {
            name,
            task,
            non_interactive,
        } => run_workflow(ctx, &name, &task, non_interactive).await,
        WorkflowCommands::Agents => run_agents(ctx).await,
    }
}

async fn run_list(ctx: &CommandContext) -> Result<()> {
    let engine = create_engine(ctx, true);

    println!("Available workflows:\n");

    for (name, description, is_custom) in engine.list_workflows() {
        let marker = if is_custom { " [custom]" } else { "" };
        println!("  {} - {}{}", name, description, marker);
    }

    Ok(())
}

async fn run_show(ctx: &CommandContext, name: &str) -> Result<()> {
    let engine = create_engine(ctx, true);

    match engine.get_workflow(name) {
        Some(workflow) => {
            println!("Workflow: {}\n", workflow.name);
            println!("Description: {}\n", workflow.description);
            println!("Steps:");

            for (i, step) in workflow.steps.iter().enumerate() {
                match step {
                    WorkflowStep::Agent {
                        name,
                        task,
                        model,
                    } => {
                        let model_info = model
                            .as_ref()
                            .map(|m| format!(" (model: {})", m))
                            .unwrap_or_default();
                        println!("  {}. Agent '{}'{}", i + 1, name, model_info);
                        println!("     Task: {}", task);
                    }
                    WorkflowStep::Checkpoint { message, show } => {
                        let show_info = show
                            .as_ref()
                            .map(|s| format!(" [shows: {}]", s))
                            .unwrap_or_default();
                        println!("  {}. Checkpoint{}", i + 1, show_info);
                        println!("     Message: {}", message);
                    }
                    _ => {
                        println!("  {}. (other step type)", i + 1);
                    }
                }
            }
        }
        None => {
            eprintln!("Error: Workflow '{}' not found", name);
            eprintln!("\nAvailable workflows:");
            for (name, description, _) in engine.list_workflows() {
                eprintln!("  {} - {}", name, description);
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn run_workflow(
    ctx: &CommandContext,
    name: &str,
    task: &str,
    non_interactive: bool,
) -> Result<()> {
    let engine = create_engine(ctx, non_interactive);

    println!("Running workflow '{}' with task: {}\n", name, task);

    match engine.run(name, task).await {
        Ok(result) => {
            println!("\nWorkflow completed with status: {:?}", result.status);
            // Show the last agent's output from context if available
            if let Some(output) = result
                .context
                .get("changes")
                .or_else(|| result.context.get("plan"))
                .or_else(|| result.context.get("review"))
            {
                println!("\nFinal output:\n{}", output);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("\nWorkflow failed: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_agents(ctx: &CommandContext) -> Result<()> {
    let registry = AgentRegistry::with_defaults(&ctx.model);

    println!("Available agents:\n");

    for (name, config) in registry.iter() {
        println!("  {} - {}", name, config.display_name);
        println!("    Model: {}", config.model);
        println!("    Temperature: {}", config.temperature);
        if !config.tools.is_empty() {
            println!("    Tools: {}", config.tools.join(", "));
        }
        if !config.can_handoff_to.is_empty() {
            println!("    Handoffs: {}", config.can_handoff_to.join(", "));
        }
        println!();
    }

    Ok(())
}

fn create_engine(ctx: &CommandContext, non_interactive: bool) -> WorkflowEngine {
    let config = EngineConfig {
        ollama_url: ctx.ollama_url.clone(),
        default_model: ctx.model.clone(),
        non_interactive,
        verbose: ctx.is_verbose(),
        custom_workflows_dir: None,
    };
    let registry = AgentRegistry::with_defaults(&ctx.model);
    WorkflowEngine::new(registry, config)
}
