//! Minimal Rust agent with Ollama and MCP support
//!
//! This is the main entry point - a slim dispatcher that routes commands
//! to their respective handlers in the `handlers` module.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use agent::cli::{Cli, Commands};
use agent::config::AgentFileConfig;
use agent::handlers::CommandContext;

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

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level.to_string()));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    let file_config = AgentFileConfig::load()?;
    let ctx = CommandContext::new(cli.ollama_url, cli.model, cli.verbose, file_config);

    dispatch(cli.command, ctx).await
}

async fn dispatch(cmd: Commands, ctx: CommandContext) -> Result<()> {
    use ::agent::handlers::*;

    match cmd {
        // =================================================================
        // Core commands - always available
        // =================================================================
        Commands::Chat { message } => core::chat(&ctx, &message).await,
        Commands::Simple => core::simple(&ctx).await,
        Commands::Models => core::models(&ctx).await,

        // =================================================================
        // MCP commands - requires "mcp" feature
        // =================================================================
        #[cfg(feature = "mcp")]
        Commands::Agent {
            message,
            system,
            servers,
        } => agent_handler::run_agent(&ctx, message, system, servers).await,

        #[cfg(feature = "mcp")]
        Commands::Tools { server } => tools::run_tools(server).await,

        #[cfg(feature = "mcp")]
        Commands::Call { tool, args } => tools::run_call_tool(&tool, args).await,

        #[cfg(feature = "mcp")]
        Commands::Serve { system } => serve::run_serve(&ctx, system).await,

        #[cfg(feature = "mcp")]
        Commands::Health {
            test_llm,
            test_tools,
            all,
        } => health::run_health(&ctx, test_llm || all, test_tools || all).await,

        #[cfg(feature = "mcp")]
        Commands::Mcps { command } => mcps::run_mcps_command(command).await,

        // =================================================================
        // Monitor commands - requires "monitor" feature
        // =================================================================
        #[cfg(feature = "monitor")]
        Commands::Monitor {
            once,
            interval,
            repos,
            system,
        } => monitor::run_monitor(&ctx, once, interval, repos, system).await,

        // =================================================================
        // Web commands - requires "web" feature
        // =================================================================
        #[cfg(feature = "web")]
        Commands::Web {
            port,
            system,
            dev,
            open,
        } => web::run_web(&ctx, port, system, dev, open).await,

        // =================================================================
        // Orchestrator commands - requires "orchestrator" feature
        // =================================================================
        #[cfg(feature = "orchestrator")]
        Commands::Workflow { command } => workflow::run_workflow_command(&ctx, command).await,
    }
}
