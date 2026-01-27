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
///
/// Set `LOG_FORMAT=json` for structured JSON output (useful for production/log aggregation).
/// Default is human-readable text output.
fn init_tracing(verbosity: u8) {
    let level = match verbosity {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level.to_string()));

    let use_json = std::env::var("LOG_FORMAT")
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    let registry = tracing_subscriber::registry().with(filter);

    if use_json {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        registry.with(tracing_subscriber::fmt::layer()).init();
    }
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

        #[cfg(feature = "orchestrator")]
        Commands::Runs { command } => runs::run_runs_command(&ctx, command).await,
    }
}
