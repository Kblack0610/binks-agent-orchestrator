//! MCP status command handler

use anyhow::Result;

use crate::mcp::McpClientPool;

/// Handle the `mcps status` command
pub async fn run_mcps_status(verbose: bool) -> Result<()> {
    println!("=== MCP Server Status ===\n");

    let pool = McpClientPool::load()?;

    match pool {
        Some(mut pool) => {
            let servers = pool.server_names();

            if servers.is_empty() {
                println!("No MCP servers configured.");
                return Ok(());
            }

            println!("Configured servers: {}\n", servers.len());

            for server in &servers {
                print!("  {} ", server);

                // Check if tools are cached
                let cached = pool.has_cached_tools(server);

                // Try to get tools (this will cache them if not already cached)
                match pool.list_tools_from(server).await {
                    Ok(tools) => {
                        let cache_status = if cached { "(cached)" } else { "(fresh)" };
                        println!("✓ {} tools {}", tools.len(), cache_status);

                        if verbose {
                            for tool in &tools {
                                let desc = tool
                                    .description
                                    .as_deref()
                                    .unwrap_or("No description")
                                    .lines()
                                    .next()
                                    .unwrap_or("");
                                // Truncate long descriptions
                                let desc = if desc.len() > 60 {
                                    format!("{}...", &desc[..60])
                                } else {
                                    desc.to_string()
                                };
                                println!("      - {} : {}", tool.name, desc);
                            }
                        }
                    }
                    Err(e) => {
                        println!("✗ Failed: {}", e);
                    }
                }
            }

            // Summary
            let all_tools = pool.list_all_tools().await?;
            println!(
                "\nTotal: {} tools across {} servers",
                all_tools.len(),
                servers.len()
            );
        }
        None => {
            println!("No .mcp.json found in current directory.");
            println!("Create one to configure MCP servers.");
        }
    }

    Ok(())
}
