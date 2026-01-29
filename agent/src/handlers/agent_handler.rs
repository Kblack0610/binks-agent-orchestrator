//! Agent command handler
//!
//! Runs the tool-using agent with MCP tools.

use anyhow::Result;

use super::CommandContext;
use crate::agent::{detect_capabilities, Agent};
use crate::cli::{Repl, ReplConfig};
use crate::mcp::{parse_model_size_with_thresholds, McpClientPool, ModelSize};
use crate::output::TerminalOutput;

/// Handle the `agent` command
pub async fn run_agent(
    ctx: &CommandContext,
    message: Option<String>,
    system: Option<String>,
    servers: Option<String>,
) -> Result<()> {
    let pool = McpClientPool::load()?
        .ok_or_else(|| anyhow::anyhow!("No .mcp.json found - agent needs MCP tools"))?;

    let server_list = CommandContext::parse_server_filter(servers);
    let effective_servers = resolve_server_filter(ctx, &pool, server_list);

    // Detect model capabilities
    let capabilities = detect_capabilities(
        &ctx.ollama_url,
        &ctx.model,
        Some(&ctx.file_config.models.overrides),
    )
    .await;

    if ctx.is_verbose() {
        tracing::info!(
            "Model capabilities for {}: tool_calling={}, thinking={}, format={:?}",
            ctx.model,
            capabilities.tool_calling,
            capabilities.thinking,
            capabilities.function_call_format
        );
    }

    let mut agent =
        Agent::from_agent_config(&ctx.ollama_url, &ctx.model, pool, &ctx.file_config.agent)
            .with_capabilities(capabilities)
            .with_verbose(ctx.is_verbose());

    // Apply system prompt
    if let Some(sys) = ctx.resolve_system_prompt(system) {
        agent = agent.with_system_prompt(&sys);
    }

    // Display info
    let tool_names = agent.tool_names().await?;
    let server_names = agent.server_names().await?;

    println!(
        "Agent mode with {} tools from {} servers",
        tool_names.len(),
        server_names.len()
    );
    println!("Servers: {}", server_names.join(", "));
    if let Some(ref filter) = effective_servers {
        println!("Filtered to: {}", filter.join(", "));
    }
    println!("Model: {}", ctx.model);
    println!();

    // Create server filter as string slices
    let server_refs: Option<Vec<&str>> = effective_servers
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());

    if let Some(msg) = message {
        // Single message mode
        println!("> {}\n", msg);
        let result = if let Some(ref srvs) = server_refs {
            agent.chat_with_servers(&msg, srvs).await
        } else {
            agent.chat(&msg).await
        };

        match result {
            Ok(response) => {
                println!("{}", response);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    } else {
        // Interactive mode with REPL
        let output = TerminalOutput::auto();

        let mut repl_config = ReplConfig::default();
        if let Some(filter) = effective_servers {
            repl_config.server_filter = Some(filter);
        }

        let mut repl = Repl::new(&mut agent, &output).with_config(repl_config);
        repl.run().await?;
    }

    Ok(())
}

/// Resolve effective server filter based on CLI, config, and model size
fn resolve_server_filter(
    ctx: &CommandContext,
    pool: &McpClientPool,
    cli_servers: Option<Vec<String>>,
) -> Option<Vec<String>> {
    // CLI override takes precedence
    if cli_servers.is_some() {
        return cli_servers;
    }

    // Auto-filter based on model size if enabled
    if ctx.file_config.mcp.auto_filter {
        let thresholds = &ctx.file_config.mcp.size_thresholds;
        let model_size =
            parse_model_size_with_thresholds(&ctx.model, thresholds.small, thresholds.medium);

        let profile = match model_size {
            ModelSize::Small | ModelSize::Unknown => &ctx.file_config.mcp.profiles.small,
            ModelSize::Medium => &ctx.file_config.mcp.profiles.medium,
            ModelSize::Large => &ctx.file_config.mcp.profiles.large,
        };

        let filtered = pool.server_names_for_profile(profile);

        if ctx.is_verbose() {
            tracing::info!(
                "Auto-filter: model={} size={:?} max_tier={} servers={:?}",
                ctx.model,
                model_size,
                profile.max_tier,
                filtered
            );
        }

        return Some(filtered);
    }

    None // Use all servers
}
