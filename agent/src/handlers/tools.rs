//! Tools command handlers
//!
//! List available tools and call tools directly.

use anyhow::Result;
use std::collections::HashMap;

use crate::mcp::McpClientPool;

/// Handle the `tools` command - list available MCP tools
pub async fn run_tools(server_filter: Option<String>) -> Result<()> {
    let pool = McpClientPool::load()?;

    match pool {
        Some(mut pool) => {
            println!("Loading MCP servers from .mcp.json...\n");

            let servers = pool.server_names();
            println!("Configured servers: {}\n", servers.join(", "));

            let tools = pool.list_all_tools().await?;

            if tools.is_empty() {
                println!("No tools found.");
                return Ok(());
            }

            // Filter by server if specified
            let tools: Vec<_> = match &server_filter {
                Some(s) => tools.into_iter().filter(|t| &t.server == s).collect(),
                None => tools,
            };

            // Group by server
            let mut by_server: HashMap<String, Vec<_>> = HashMap::new();
            for tool in tools {
                by_server.entry(tool.server.clone()).or_default().push(tool);
            }

            for (server, tools) in by_server {
                println!("=== {} ({} tools) ===", server, tools.len());
                for tool in tools {
                    let desc = tool
                        .description
                        .as_deref()
                        .unwrap_or("No description")
                        .lines()
                        .next()
                        .unwrap_or("");
                    println!("  {} - {}", tool.name, desc);
                }
                println!();
            }
        }
        None => {
            println!("No .mcp.json found in current directory or parents.");
            println!("Create one to configure MCP servers.");
        }
    }

    Ok(())
}

/// Handle the `call` command - call a tool directly
pub async fn run_call_tool(tool_name: &str, args: Option<String>) -> Result<()> {
    let mut pool = McpClientPool::load()?.ok_or_else(|| anyhow::anyhow!("No .mcp.json found"))?;

    let arguments = match args {
        Some(json) => Some(serde_json::from_str(&json)?),
        None => None,
    };

    println!("Calling tool: {}", tool_name);
    if let Some(ref a) = arguments {
        println!("Arguments: {}", serde_json::to_string_pretty(a)?);
    }
    println!();

    let result = pool.call_tool(tool_name, arguments).await?;

    println!("Result:");
    for content in &result.content {
        match &content.raw {
            rmcp::model::RawContent::Text(text) => {
                println!("{}", text.text);
            }
            _ => {
                println!("{:?}", content);
            }
        }
    }

    Ok(())
}
