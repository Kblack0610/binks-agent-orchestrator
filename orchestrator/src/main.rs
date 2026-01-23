//! Orchestrator CLI
//!
//! Provides workflow execution commands for multi-agent flows.
//!
//! Usage:
//!   orchestrator workflow run implement-feature --task "Add dark mode"
//!   orchestrator workflow list
//!   orchestrator workflow show implement-feature
//!   orchestrator agents list
//!   orchestrator agents show planner

use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use agent::config::AgentFileConfig;
use orchestrator::agent_config::AgentRegistry;
use orchestrator::engine::{EngineConfig, WorkflowEngine};

#[derive(Parser)]
#[command(name = "orchestrator")]
#[command(about = "Multi-agent workflow orchestration for binks-agent")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Ollama server URL
    #[arg(long, env = "OLLAMA_URL", global = true)]
    ollama_url: Option<String>,

    /// Default model to use
    #[arg(short = 'm', long, env = "OLLAMA_MODEL", global = true)]
    model: Option<String>,

    /// Increase verbosity (-v info, -vv debug, -vvv trace). Default is warn.
    #[arg(short, long, action = ArgAction::Count, global = true)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Workflow management and execution
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommands,
    },
    /// Agent management
    Agents {
        #[command(subcommand)]
        command: AgentCommands,
    },
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// Run a workflow
    Run {
        /// Workflow name (e.g., "implement-feature", "fix-bug")
        workflow: String,

        /// Task description
        #[arg(long, short)]
        task: String,

        /// Directory for custom workflow files
        #[arg(long)]
        workflows_dir: Option<PathBuf>,

        /// Run in non-interactive mode (auto-approve checkpoints)
        #[arg(long)]
        non_interactive: bool,
    },
    /// List available workflows
    List,
    /// Show workflow definition
    Show {
        /// Workflow name
        workflow: String,
    },
}

#[derive(Subcommand)]
enum AgentCommands {
    /// List available agents
    List,
    /// Show agent configuration
    Show {
        /// Agent name
        agent: String,
    },
}

/// Initialize tracing with the given verbosity level
///
/// - 0: warn (default)
/// - 1: info (-v)
/// - 2: debug (-vv)
/// - 3+: trace (-vvv)
fn init_tracing(verbosity: u8) {
    let level = match verbosity {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    // Allow RUST_LOG to override if set
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level.to_string()));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI first to get verbosity before initializing tracing
    let cli = Cli::parse();

    // Initialize tracing with CLI-controlled verbosity
    init_tracing(cli.verbose);

    // Print deprecation warning (only at info level or higher)
    if cli.verbose >= 1 {
        eprintln!("\x1b[33m");
        eprintln!("╔════════════════════════════════════════════════════════════════╗");
        eprintln!("║  DEPRECATION NOTICE                                             ║");
        eprintln!("║                                                                  ║");
        eprintln!("║  The standalone 'orchestrator' binary is deprecated.            ║");
        eprintln!("║  Please use 'agent workflow' commands instead:                   ║");
        eprintln!("║                                                                  ║");
        eprintln!("║    agent workflow list                                           ║");
        eprintln!("║    agent workflow show <name>                                    ║");
        eprintln!("║    agent workflow run <name> --task \"description\"              ║");
        eprintln!("║    agent workflow agents                                         ║");
        eprintln!("║                                                                  ║");
        eprintln!("║  This binary will be removed in a future release.               ║");
        eprintln!("╚════════════════════════════════════════════════════════════════╝");
        eprintln!("\x1b[0m");
    }

    // Load config
    let file_config = AgentFileConfig::load()?;

    // Resolve config values
    let ollama_url = cli
        .ollama_url
        .unwrap_or_else(|| file_config.llm.url.clone());
    let model = cli.model.unwrap_or_else(|| file_config.llm.model.clone());

    match cli.command {
        Commands::Workflow { command } => {
            run_workflow_command(command, &ollama_url, &model, cli.verbose >= 1).await
        }
        Commands::Agents { command } => run_agents_command(command, &model),
    }
}

async fn run_workflow_command(
    command: WorkflowCommands,
    ollama_url: &str,
    model: &str,
    verbose: bool,
) -> Result<()> {
    match command {
        WorkflowCommands::Run {
            workflow,
            task,
            workflows_dir,
            non_interactive,
        } => {
            let registry = AgentRegistry::with_defaults(model);
            let config = EngineConfig {
                ollama_url: ollama_url.to_string(),
                default_model: model.to_string(),
                custom_workflows_dir: workflows_dir,
                non_interactive,
                verbose,
            };

            let engine = WorkflowEngine::new(registry, config);

            let result = engine.run(&workflow, &task).await?;

            // Print final status
            println!("\nWorkflow Status: {:?}", result.status);
            println!(
                "Steps completed: {}/{}",
                result.step_results.len(),
                result.step_results.len()
            );

            let total_duration: u64 = result.step_results.iter().map(|s| s.duration_ms).sum();
            println!("Total duration: {}ms", total_duration);
        }

        WorkflowCommands::List => {
            let registry = AgentRegistry::with_defaults(model);
            let config = EngineConfig::default();
            let engine = WorkflowEngine::new(registry, config);

            println!("Available Workflows:\n");

            let workflows = engine.list_workflows();
            let mut builtins = Vec::new();
            let mut customs = Vec::new();

            for (name, desc, is_custom) in workflows {
                if is_custom {
                    customs.push((name, desc));
                } else {
                    builtins.push((name, desc));
                }
            }

            println!("Built-in:");
            for (name, desc) in builtins {
                println!("  {} - {}", name, desc);
            }

            if !customs.is_empty() {
                println!("\nCustom:");
                for (name, desc) in customs {
                    println!("  {} - {}", name, desc);
                }
            }

            println!(
                "\nRun a workflow with: orchestrator workflow run <name> --task \"description\""
            );
        }

        WorkflowCommands::Show { workflow } => {
            let registry = AgentRegistry::with_defaults(model);
            let config = EngineConfig::default();
            let engine = WorkflowEngine::new(registry, config);

            match engine.get_workflow(&workflow) {
                Some(wf) => {
                    println!("Workflow: {}\n", wf.name);
                    if !wf.description.is_empty() {
                        println!("Description: {}\n", wf.description);
                    }
                    println!("Steps:");
                    for (i, step) in wf.steps.iter().enumerate() {
                        match step {
                            orchestrator::WorkflowStep::Agent { name, task, model } => {
                                println!("  {}. [Agent: {}]", i + 1, name);
                                println!("     Task: {}", task);
                                if let Some(m) = model {
                                    println!("     Model: {}", m);
                                }
                            }
                            orchestrator::WorkflowStep::Checkpoint { message, show } => {
                                println!("  {}. [Checkpoint]", i + 1);
                                println!("     Message: {}", message);
                                if let Some(s) = show {
                                    println!("     Show: {}", s);
                                }
                            }
                            orchestrator::WorkflowStep::Parallel(steps) => {
                                println!("  {}. [Parallel: {} steps]", i + 1, steps.len());
                            }
                            orchestrator::WorkflowStep::Branch { condition, .. } => {
                                println!("  {}. [Branch: {}]", i + 1, condition);
                            }
                        }
                    }
                }
                None => {
                    eprintln!("Workflow '{}' not found.", workflow);
                    eprintln!("Use 'orchestrator workflow list' to see available workflows.");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

fn run_agents_command(command: AgentCommands, model: &str) -> Result<()> {
    let registry = AgentRegistry::with_defaults(model);

    match command {
        AgentCommands::List => {
            println!("Available Agents:\n");
            for (name, config) in registry.iter() {
                println!("  {} ({}) - {}", name, config.model, config.display_name);
                if !config.tools.is_empty() {
                    println!("    Tools: {}", config.tools.join(", "));
                }
            }
        }

        AgentCommands::Show { agent } => match registry.get(&agent) {
            Some(config) => {
                println!("Agent: {}\n", config.name);
                println!("Display Name: {}", config.display_name);
                println!("Model: {}", config.model);
                println!("Temperature: {}", config.temperature);
                if let Some(max) = config.max_tokens {
                    println!("Max Tokens: {}", max);
                }
                if !config.tools.is_empty() {
                    println!("Tools: {}", config.tools.join(", "));
                }
                if !config.can_handoff_to.is_empty() {
                    println!("Can hand off to: {}", config.can_handoff_to.join(", "));
                }
                println!("\nSystem Prompt:\n{}", config.system_prompt);
            }
            None => {
                eprintln!("Agent '{}' not found.", agent);
                eprintln!("Use 'orchestrator agents list' to see available agents.");
                std::process::exit(1);
            }
        },
    }

    Ok(())
}
