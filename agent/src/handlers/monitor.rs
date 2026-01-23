//! Monitor command handler
//!
//! Run the repository monitoring agent.

use anyhow::Result;

use super::CommandContext;
use crate::monitor::{self, MonitorConfig};

/// Handle the `monitor` command
pub async fn run_monitor(
    ctx: &CommandContext,
    once: bool,
    interval: u64,
    repos: Option<String>,
    system: Option<String>,
) -> Result<()> {
    let repos = repos
        .map(|r| r.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_else(|| ctx.file_config.monitor.repos.clone());

    if repos.is_empty() {
        eprintln!("Error: No repositories specified. Use --repos to specify repos to monitor,");
        eprintln!("or set repos in .agent.toml under [monitor].");
        eprintln!("Example: agent monitor --once --repos owner/repo1,owner/repo2");
        std::process::exit(1);
    }

    let config = MonitorConfig {
        ollama_url: ctx.ollama_url.clone(),
        model: ctx.model.clone(),
        repos,
        once,
        interval,
        system_prompt: system,
    };

    monitor::run_monitor(config).await
}
