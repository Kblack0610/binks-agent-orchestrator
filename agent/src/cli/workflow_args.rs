//! Workflow orchestrator subcommands
//!
//! Commands for running multi-agent workflows.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum WorkflowCommands {
    /// List available workflows
    List,
    /// Show workflow details
    Show {
        /// Name of the workflow to show
        name: String,
    },
    /// Run a workflow
    Run {
        /// Name of the workflow to run
        name: String,
        /// Task description
        #[arg(long, short)]
        task: String,
        /// Run without human checkpoints (auto-approve all)
        #[arg(long)]
        non_interactive: bool,
    },
    /// List available agents
    Agents,
}
