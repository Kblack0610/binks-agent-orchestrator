//! Health check command handler
//!
//! Runs health checks on agent components.

use anyhow::Result;
use std::collections::HashMap;

use super::CommandContext;
use crate::config::{AgentFileConfig, McpConfig};
use crate::llm::{Llm, OllamaClient};
use crate::mcp::McpClientPool;

/// Handle the `health` command
pub async fn run_health(ctx: &CommandContext, test_llm: bool, test_tools: bool) -> Result<()> {
    println!("=== Agent Health Check ===\n");

    let mut all_passed = true;
    let mut checks_run = 0;
    let mut checks_passed = 0;

    // 1. Check .agent.toml config
    checks_run += 1;
    if check_agent_config(ctx, &mut checks_passed) {
        // Config check always passes (uses defaults if not found)
    } else {
        all_passed = false;
    }

    // 2. Check .mcp.json config
    checks_run += 1;
    if !check_mcp_config(&mut checks_passed) {
        all_passed = false;
    }

    // 3. Check MCP server connections
    checks_run += 1;
    if !check_mcp_connections(&mut checks_passed).await {
        all_passed = false;
    }

    // 4. Optional: Test LLM connectivity
    if test_llm {
        checks_run += 1;
        if !check_llm_connectivity(ctx, &mut checks_passed).await {
            all_passed = false;
        }
    }

    // 5. Optional: Test tool execution
    if test_tools {
        checks_run += 1;
        if !check_tool_execution(&mut checks_passed).await {
            all_passed = false;
        }
    }

    // Summary
    println!("\n=== Summary ===");
    println!("Checks: {}/{} passed", checks_passed, checks_run);

    if all_passed {
        println!("\nAll health checks passed!");
        Ok(())
    } else {
        println!("\nSome health checks failed.");
        std::process::exit(1);
    }
}

fn status(passed: bool) -> &'static str {
    if passed {
        "✓"
    } else {
        "✗"
    }
}

fn check_agent_config(ctx: &CommandContext, checks_passed: &mut u32) -> bool {
    print!("Config (.agent.toml): ");
    let config_path = std::env::current_dir()
        .map(|p| p.join(".agent.toml"))
        .unwrap_or_default();

    if config_path.exists() {
        match AgentFileConfig::load() {
            Ok(config) => {
                println!("{} Found", status(true));
                println!("  - LLM URL: {}", config.llm.url);
                println!("  - Model: {}", config.llm.model);
                *checks_passed += 1;
                true
            }
            Err(e) => {
                println!("{} Parse error: {}", status(false), e);
                false
            }
        }
    } else {
        println!("{} Not found (using defaults)", status(true));
        println!("  - LLM URL: {}", ctx.ollama_url);
        println!("  - Model: {}", ctx.model);
        *checks_passed += 1;
        true
    }
}

fn check_mcp_config(checks_passed: &mut u32) -> bool {
    print!("Config (.mcp.json): ");
    let mcp_path = std::env::current_dir()
        .map(|p| p.join(".mcp.json"))
        .unwrap_or_default();

    if mcp_path.exists() {
        match McpConfig::load() {
            Ok(Some(config)) => {
                let server_count = config.mcp_servers.len();
                println!("{} Found ({} servers)", status(true), server_count);
                for name in config.mcp_servers.keys() {
                    println!("  - {}", name);
                }
                *checks_passed += 1;
                true
            }
            Ok(None) => {
                println!("{} Not found", status(false));
                false
            }
            Err(e) => {
                println!("{} Parse error: {}", status(false), e);
                false
            }
        }
    } else {
        println!("{} Not found", status(false));
        false
    }
}

async fn check_mcp_connections(checks_passed: &mut u32) -> bool {
    print!("MCP Connections: ");
    match McpClientPool::load() {
        Ok(Some(mut pool)) => match pool.list_all_tools().await {
            Ok(tools) => {
                let mut by_server: HashMap<String, usize> = HashMap::new();
                for tool in &tools {
                    *by_server.entry(tool.server.clone()).or_default() += 1;
                }

                let connected_servers = by_server.len();
                let total_tools = tools.len();
                println!(
                    "{} {} servers, {} tools",
                    status(true),
                    connected_servers,
                    total_tools
                );

                for (server, count) in by_server.iter() {
                    println!("  - {}: {} tools", server, count);
                }
                *checks_passed += 1;
                true
            }
            Err(e) => {
                println!("{} Tool discovery failed: {}", status(false), e);
                false
            }
        },
        Ok(None) => {
            println!("{} No .mcp.json found", status(false));
            false
        }
        Err(e) => {
            println!("{} Failed to load: {}", status(false), e);
            false
        }
    }
}

async fn check_llm_connectivity(ctx: &CommandContext, checks_passed: &mut u32) -> bool {
    print!("LLM Connectivity: ");
    let llm = OllamaClient::new(&ctx.ollama_url, &ctx.model);
    match llm.chat("Say 'OK' and nothing else.").await {
        Ok(response) => {
            let trimmed = response.trim();
            let short_response = if trimmed.len() > 50 {
                format!("{}...", &trimmed[..50])
            } else {
                trimmed.to_string()
            };
            println!("{} Response: \"{}\"", status(true), short_response);
            *checks_passed += 1;
            true
        }
        Err(e) => {
            println!("{} Failed: {}", status(false), e);
            false
        }
    }
}

async fn check_tool_execution(checks_passed: &mut u32) -> bool {
    print!("Tool Execution: ");
    match McpClientPool::load() {
        Ok(Some(mut pool)) => match pool.call_tool("get_uptime", None).await {
            Ok(result) => {
                let text = result
                    .content
                    .iter()
                    .filter_map(|c| match &c.raw {
                        rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .next()
                    .unwrap_or("(no text)");

                let short_text = if text.len() > 60 {
                    format!("{}...", &text[..60])
                } else {
                    text.to_string()
                };
                println!("{} get_uptime returned: {}", status(true), short_text);
                *checks_passed += 1;
                true
            }
            Err(e) => {
                println!("{} Failed: {}", status(false), e);
                false
            }
        },
        Ok(None) => {
            println!("{} No MCP pool available", status(false));
            false
        }
        Err(e) => {
            println!("{} Pool load failed: {}", status(false), e);
            false
        }
    }
}
