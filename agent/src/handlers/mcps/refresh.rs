//! MCP refresh command handler

use anyhow::Result;

use crate::mcp::McpClientPool;

/// Handle the `mcps refresh` command
pub async fn run_mcps_refresh() -> Result<()> {
    println!("Refreshing MCP connections...\n");

    let pool = McpClientPool::load()?;

    match pool {
        Some(mut pool) => {
            // Clear cache
            pool.clear_cache();
            println!("Cache cleared.");

            // Reconnect to all servers
            let servers = pool.server_names();
            let mut success = 0;
            let mut failed = 0;

            for server in &servers {
                print!("  {} ", server);
                match pool.list_tools_from(server).await {
                    Ok(tools) => {
                        println!("✓ {} tools", tools.len());
                        success += 1;
                    }
                    Err(e) => {
                        println!("✗ {}", e);
                        failed += 1;
                    }
                }
            }

            println!(
                "\nRefresh complete: {} succeeded, {} failed",
                success, failed
            );
        }
        None => {
            println!("No .mcp.json found.");
        }
    }

    Ok(())
}
