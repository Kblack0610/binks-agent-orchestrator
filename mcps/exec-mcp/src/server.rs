//! MCP Server implementation for sandboxed command execution
//!
//! This module defines the main MCP server that exposes command execution as tools.
//! Handler implementations are in the handlers module.

use std::path::PathBuf;

use mcp_common::{CallToolResult, McpError};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::guard::CommandGuard;
use crate::handlers;
use crate::params::*;
use crate::types::{Config, ExecError};

/// The Exec MCP Server
#[derive(Clone)]
pub struct ExecMcpServer {
    guard: CommandGuard,
    config: Config,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Tool Router - Each tool delegates to its handler
// ============================================================================

#[tool_router]
impl ExecMcpServer {
    /// Create a new server, loading config from standard locations
    ///
    /// Config is searched in order:
    /// 1. `EXEC_CONFIG_PATH` env var
    /// 2. `~/.binks/exec.toml`
    /// 3. `./exec-mcp.toml`
    /// 4. `$XDG_CONFIG_HOME/exec-mcp/config.toml`
    /// 5. `~/.exec-mcp.toml`
    /// 6. Default config if none found
    pub fn new() -> Self {
        Self::with_config(Self::load_config()).expect("Failed to create ExecMcpServer")
    }

    /// Create a new server with explicit config
    pub fn with_config(config: Config) -> Result<Self, ExecError> {
        let guard = CommandGuard::new(&config)?;

        Ok(Self {
            guard,
            config,
            tool_router: Self::tool_router(),
        })
    }

    /// Load config from standard file locations
    fn load_config() -> Config {
        // 1. Check EXEC_CONFIG_PATH env var first
        if let Ok(env_path) = std::env::var("EXEC_CONFIG_PATH") {
            let path = PathBuf::from(&env_path);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    match toml::from_str::<Config>(&content) {
                        Ok(config) => {
                            tracing::info!(
                                "Loaded config from EXEC_CONFIG_PATH={}",
                                path.display()
                            );
                            return config;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse config from EXEC_CONFIG_PATH={}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            } else {
                tracing::warn!("EXEC_CONFIG_PATH={} does not exist", env_path);
            }
        }

        // 2-5. Check standard file locations
        let mut config_paths = Vec::new();

        // 2. ~/.binks/exec.toml (project convention)
        if let Some(home) = dirs::home_dir() {
            config_paths.push(home.join(".binks").join("exec.toml"));
        }

        // 3. ./exec-mcp.toml (local override)
        config_paths.push(PathBuf::from("exec-mcp.toml"));

        // 4. $XDG_CONFIG_HOME/exec-mcp/config.toml
        if let Some(config_dir) = dirs::config_dir() {
            config_paths.push(config_dir.join("exec-mcp").join("config.toml"));
        }

        // 5. ~/.exec-mcp.toml
        if let Some(home) = dirs::home_dir() {
            config_paths.push(home.join(".exec-mcp.toml"));
        }

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

        // 6. Default config
        tracing::info!("Using default configuration");
        Config::default()
    }

    #[tool(description = "Execute a shell command with default timeout")]
    async fn run_command(
        &self,
        Parameters(params): Parameters<RunCommandParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::run_command(&self.guard, &self.config, params).await
    }

    #[tool(description = "Execute a shell command with explicit timeout (clamped to server max)")]
    async fn run_command_with_timeout(
        &self,
        Parameters(params): Parameters<RunCommandWithTimeoutParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::run_command_with_timeout(&self.guard, &self.config, params).await
    }

    #[tool(description = "Execute a multi-line script via the configured shell")]
    async fn run_script(
        &self,
        Parameters(params): Parameters<RunScriptParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::run_script(&self.guard, &self.config, params).await
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for ExecMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Sandboxed command execution MCP server with security controls. \
                 Commands are validated against allow/deny lists. \
                 Working directories are restricted to configured paths."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for ExecMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
