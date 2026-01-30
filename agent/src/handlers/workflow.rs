//! Workflow command handler
//!
//! Run multi-agent workflows.

use anyhow::{Context, Result};
use std::collections::HashMap;

use super::CommandContext;
use crate::cli::WorkflowCommands;
use crate::workflow_client::WorkflowClient;

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
    let pool = ctx.mcp_pool_required().await?;
    let mut client = WorkflowClient::new(pool);

    let workflows = client
        .list_workflows()
        .await
        .context("Failed to list workflows")?;

    println!("Available workflows:\n");

    for workflow in workflows {
        println!("  {} - {}", workflow.name, workflow.description);
        println!("    Steps: {}", workflow.step_count);
    }

    Ok(())
}

async fn run_show(ctx: &CommandContext, name: &str) -> Result<()> {
    let pool = ctx.mcp_pool_required().await?;
    let mut client = WorkflowClient::new(pool);

    let workflows = client
        .list_workflows()
        .await
        .context("Failed to list workflows")?;

    match workflows.iter().find(|w| w.name == name) {
        Some(workflow) => {
            println!("Workflow: {}\n", workflow.name);
            println!("Description: {}\n", workflow.description);
            println!("Total steps: {}\n", workflow.step_count);
            println!(
                "Run with: agent workflow run {} \"<your task description>\"",
                workflow.name
            );
        }
        None => {
            eprintln!("Error: Workflow '{}' not found", name);
            eprintln!("\nAvailable workflows:");
            for w in workflows {
                eprintln!("  {} - {}", w.name, w.description);
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
    _non_interactive: bool,
) -> Result<()> {
    let pool = ctx.mcp_pool_required().await?;
    let mut client = WorkflowClient::new(pool);

    println!("Running workflow '{}' with task: {}\n", name, task);

    // Start workflow execution
    let execution_id = client
        .execute_workflow(name, task, HashMap::new())
        .await
        .context("Failed to start workflow execution")?;

    println!("Workflow started with execution ID: {}\n", execution_id);

    // Poll for completion
    loop {
        let status = client
            .get_execution_status(&execution_id)
            .await
            .context("Failed to get execution status")?;

        println!(
            "Status: {} (step {}/{})",
            status.status, status.current_step, status.completed_steps
        );

        match status.status.as_str() {
            "completed" => {
                println!("\nWorkflow completed successfully!");
                // Show the last agent's output from context if available
                if let Some(output) = status
                    .context
                    .get("changes")
                    .or_else(|| status.context.get("plan"))
                    .or_else(|| status.context.get("review"))
                {
                    println!("\nFinal output:\n{}", output);
                }
                return Ok(());
            }
            "failed" => {
                eprintln!("\nWorkflow failed");
                std::process::exit(1);
            }
            "waiting_for_approval" => {
                // TODO: Handle checkpoint approval in interactive mode
                println!("\nWorkflow paused at checkpoint - approval needed");
                // For now, auto-approve
                client
                    .resume_from_checkpoint(&execution_id, true, None)
                    .await
                    .context("Failed to resume from checkpoint")?;
            }
            _ => {
                // Still running, wait and poll again
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
}

async fn run_agents(_ctx: &CommandContext) -> Result<()> {
    // TODO: This command will be replaced by a skill in Phase 4
    // For now, return an error directing users to use workflow-mcp directly
    eprintln!("The 'agents' command has been moved to workflow-mcp.");
    eprintln!("This command will be replaced by a skill in a future update.");
    std::process::exit(1);
}
