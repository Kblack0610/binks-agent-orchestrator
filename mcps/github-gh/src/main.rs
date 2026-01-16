//! GitHub CLI MCP Server
//!
//! This server wraps the GitHub CLI (`gh`) to provide MCP-compatible tools
//! for interacting with GitHub Enterprise via OAuth authentication.
//!
//! # Features
//!
//! - **Issues**: List, view, create, edit, and close issues
//! - **Pull Requests**: List, view, create, and merge PRs
//! - **Workflows**: List workflows, trigger runs, view run status
//! - **Repositories**: List and view repository information
//!
//! # Requirements
//!
//! - GitHub CLI (`gh`) must be installed and in PATH
//! - `gh` must be authenticated (`gh auth login`)
//!
//! # Usage
//!
//! Run directly:
//! ```bash
//! github-gh-mcp
//! ```
//!
//! Or configure in `.mcp.json`:
//! ```json
//! {
//!   "mcpServers": {
//!     "github-gh": {
//!       "command": "./mcps/github-gh/target/release/github-gh-mcp"
//!     }
//!   }
//! }
//! ```

use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod gh;
mod server;
mod tools;
mod types;

use server::GitHubMcpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (stdout is used for MCP protocol)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .with(EnvFilter::from_default_env().add_directive("github_gh_mcp=info".parse()?))
        .init();

    tracing::info!("Starting GitHub CLI MCP Server");

    // Check gh availability (optional startup check)
    if let Err(e) = gh::check_gh_available().await {
        tracing::warn!("gh CLI check failed: {}", e);
        // Continue anyway - errors will be reported per-tool
    }

    // Create the MCP server with all tools
    let server = GitHubMcpServer::new();

    // Create stdio transport and serve
    let service = server.serve(stdio()).await?;

    tracing::info!("Server running, waiting for requests...");

    // Wait for shutdown
    service.waiting().await?;

    tracing::info!("Server shutting down");
    Ok(())
}
