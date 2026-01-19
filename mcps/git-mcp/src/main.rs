//! Git MCP - Local git operations server using libgit2
//!
//! Provides git repository operations that complement GitHub API tools.
//! Useful for local repo inspection, diffs, blame, and history analysis.

mod server;
mod types;

use rmcp::{transport::io::stdio, ServiceExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use server::GitMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (MCP uses stdio for protocol)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .with(EnvFilter::from_default_env().add_directive("git_mcp=info".parse()?))
        .init();

    tracing::info!("Starting Git MCP server");

    // Create the server
    let server = GitMcpServer::new();

    // Start serving on stdio
    let service = server.serve(stdio()).await?;

    tracing::info!("Git MCP server running");

    // Wait for shutdown
    service.waiting().await?;

    tracing::info!("Git MCP server stopped");

    Ok(())
}
