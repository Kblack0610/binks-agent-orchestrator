//! System Info MCP Server
//!
//! This server provides cross-platform system information tools via the
//! Model Context Protocol (MCP). It can retrieve OS details, CPU/memory stats,
//! disk usage, network interfaces, and system uptime.
//!
//! # Features
//!
//! - **OS Info**: Name, version, kernel, hostname, architecture
//! - **CPU**: Model, cores, frequency, usage
//! - **Memory**: Total, used, available RAM and swap
//! - **Disk**: Partitions, mount points, filesystem, usage
//! - **Network**: Interfaces, IPs, MACs, traffic stats
//! - **Uptime**: Seconds and human-readable format
//!
//! # Platforms
//!
//! Works on Linux, macOS, and Windows.
//!
//! # Usage
//!
//! Run directly:
//! ```bash
//! sysinfo-mcp
//! ```
//!
//! Or configure in `.mcp.json`:
//! ```json
//! {
//!   "mcpServers": {
//!     "sysinfo": {
//!       "command": "./mcps/sysinfo-mcp/target/release/sysinfo-mcp"
//!     }
//!   }
//! }
//! ```

use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod info;
mod server;
mod types;

use server::SysInfoMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (stdout is used for MCP protocol)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .with(EnvFilter::from_default_env().add_directive("sysinfo_mcp=info".parse()?))
        .init();

    tracing::info!("Starting System Info MCP Server");

    // Create the MCP server with all tools
    let server = SysInfoMcpServer::new();

    // Create stdio transport and serve
    let service = server.serve(stdio()).await?;

    tracing::info!("Server running, waiting for requests...");

    // Wait for shutdown
    service.waiting().await?;

    tracing::info!("Server shutting down");
    Ok(())
}
