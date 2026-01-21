//! Git MCP - Local git operations server using libgit2
//!
//! Provides git repository operations that complement GitHub API tools.
//! Useful for local repo inspection, diffs, blame, and history analysis.

mod server;
mod types;

use rmcp::{transport::io::stdio, ServiceExt};
use server::GitMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp_common::init_tracing("git_mcp")?;

    tracing::info!("Starting Git MCP server");

    let server = GitMcpServer::new();
    let service = server.serve(stdio()).await?;

    tracing::info!("Git MCP server running");

    service.waiting().await?;

    tracing::info!("Git MCP server stopped");

    Ok(())
}
