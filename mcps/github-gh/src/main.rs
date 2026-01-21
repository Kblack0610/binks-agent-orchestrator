//! GitHub CLI MCP Server
//!
//! MCP-compatible tools for GitHub via the `gh` CLI.
//!
//! # Features
//! - Issues: List, view, create, edit, close
//! - Pull Requests: List, view, create, merge
//! - Workflows: List, trigger, view status
//! - Repositories: List and view
//!
//! # Requirements
//! - `gh` CLI installed and authenticated (`gh auth login`)

use rmcp::{transport::stdio, ServiceExt};

mod gh;
mod server;
mod tools;
mod types;

use server::GitHubMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp_common::init_tracing("github_gh_mcp")?;

    tracing::info!("Starting GitHub CLI MCP Server");

    // Check gh CLI availability (optional startup check)
    if let Err(e) = gh::check_gh_available().await {
        tracing::warn!("gh CLI check failed: {}", e);
    }

    let server = GitHubMcpServer::new();
    let service = server.serve(stdio()).await?;

    tracing::info!("Server running, waiting for requests...");
    service.waiting().await?;

    tracing::info!("Server shutting down");
    Ok(())
}
