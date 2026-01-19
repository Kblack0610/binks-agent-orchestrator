//! Filesystem MCP - Sandboxed filesystem server with security controls
//!
//! Provides secure file operations with configurable allowlists/denylists.
//! Operations are restricted to configured directories to prevent unauthorized access.

mod sandbox;
mod server;
mod types;

use std::path::PathBuf;

use rmcp::{transport::io::stdio, ServiceExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use server::FilesystemMcpServer;
use types::Config;

fn load_config() -> Config {
    // Try to load config from standard locations
    let config_paths = [
        // 1. Current directory
        PathBuf::from("filesystem-mcp.toml"),
        // 2. User config directory
        dirs::config_dir()
            .map(|p| p.join("filesystem-mcp").join("config.toml"))
            .unwrap_or_default(),
        // 3. Home directory
        dirs::home_dir()
            .map(|p| p.join(".filesystem-mcp.toml"))
            .unwrap_or_default(),
    ];

    for path in config_paths {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                match toml::from_str::<Config>(&content) {
                    Ok(config) => {
                        tracing::info!("Loaded config from {}", path.display());
                        return config;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse config {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    tracing::info!("Using default configuration");
    Config::default()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (MCP uses stdio for protocol)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .with(EnvFilter::from_default_env().add_directive("filesystem_mcp=info".parse()?))
        .init();

    tracing::info!("Starting Filesystem MCP server");

    // Load configuration
    let config = load_config();

    // Create the server
    let server = FilesystemMcpServer::new(config)?;

    // Start serving on stdio
    let service = server.serve(stdio()).await?;

    tracing::info!("Filesystem MCP server running");

    // Wait for shutdown
    service.waiting().await?;

    tracing::info!("Filesystem MCP server stopped");

    Ok(())
}
