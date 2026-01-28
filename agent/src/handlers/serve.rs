//! Serve command handler
//!
//! Run the agent as an MCP server.

use anyhow::Result;

use super::CommandContext;
use crate::server::{self, ServerConfig};

/// Handle the `serve` command - run as MCP server
pub async fn run_serve(ctx: &CommandContext, system: Option<String>) -> Result<()> {
    let config = ServerConfig {
        ollama_url: ctx.ollama_url.clone(),
        model: ctx.model.clone(),
        system_prompt: ctx.resolve_system_prompt(system),
        enable_runs: true,
        agent_config: ctx.file_config.agent.clone(),
    };
    server::serve(config).await
}
