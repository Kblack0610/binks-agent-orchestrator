//! Filesystem MCP - Sandboxed filesystem server with security controls
//!
//! Provides secure file operations with configurable allowlists/denylists.
//! Operations are restricted to configured directories.

mod handlers;
mod params;
mod sandbox;
mod server;
mod types;

use std::path::PathBuf;

use rmcp::{transport::io::stdio, ServiceExt};

use server::FilesystemMcpServer;
use types::Config;

fn load_config() -> Config {
    let config_paths = [
        PathBuf::from("filesystem-mcp.toml"),
        dirs::config_dir()
            .map(|p| p.join("filesystem-mcp").join("config.toml"))
            .unwrap_or_default(),
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
    mcp_common::init_tracing("filesystem_mcp")?;

    tracing::info!("Starting Filesystem MCP server");

    let config = load_config();
    let server = FilesystemMcpServer::new(config)?;
    let service = server.serve(stdio()).await?;

    tracing::info!("Filesystem MCP server running");
    service.waiting().await?;

    tracing::info!("Filesystem MCP server stopped");
    Ok(())
}
