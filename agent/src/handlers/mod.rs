//! Command handlers module
//!
//! This module contains handler functions for CLI commands, organized by feature.
//! CommandContext provides lazy-loaded resources shared across handlers.

use anyhow::Result;
use tokio::sync::OnceCell;

use crate::config::AgentFileConfig;
use crate::llm::OllamaClient;

// =============================================================================
// Always available - core handlers
// =============================================================================
pub mod core;

pub use core::{chat, models, simple};

// =============================================================================
// MCP feature handlers
// =============================================================================
#[cfg(feature = "mcp")]
pub mod agent_handler;
#[cfg(feature = "mcp")]
pub mod health;
#[cfg(feature = "mcp")]
pub mod mcps;
#[cfg(feature = "mcp")]
pub mod serve;
#[cfg(feature = "mcp")]
pub mod tools;

#[cfg(feature = "mcp")]
pub use agent_handler::run_agent;
#[cfg(feature = "mcp")]
pub use health::run_health;
#[cfg(feature = "mcp")]
pub use mcps::run_mcps_command;
#[cfg(feature = "mcp")]
pub use serve::run_serve;
#[cfg(feature = "mcp")]
pub use tools::{run_call_tool, run_tools};

// =============================================================================
// Monitor feature handlers
// =============================================================================
#[cfg(feature = "monitor")]
pub mod monitor;

#[cfg(feature = "monitor")]
pub use monitor::run_monitor;

// =============================================================================
// Web feature handlers
// =============================================================================
#[cfg(feature = "web")]
pub mod web;

#[cfg(feature = "web")]
pub use web::run_web;

// =============================================================================
// Orchestrator feature handlers
// =============================================================================
#[cfg(feature = "orchestrator")]
pub mod runs;
#[cfg(feature = "orchestrator")]
pub mod selfheal;
#[cfg(feature = "orchestrator")]
pub mod workflow;

#[cfg(feature = "orchestrator")]
pub use runs::run_runs_command;
#[cfg(feature = "orchestrator")]
pub use selfheal::run_selfheal_command;
#[cfg(feature = "orchestrator")]
pub use workflow::run_workflow_command;

// =============================================================================
// CommandContext - shared state with lazy-loading
// =============================================================================

/// Shared context for command handlers with lazy-loaded resources.
///
/// Resources like MCP pool are only initialized when first accessed,
/// avoiding startup overhead for commands that don't need them.
pub struct CommandContext {
    pub ollama_url: String,
    pub model: String,
    pub verbose: u8,
    pub file_config: AgentFileConfig,

    /// Lazy-loaded MCP client pool
    #[cfg(feature = "mcp")]
    mcp_pool: OnceCell<Option<crate::mcp::McpClientPool>>,
}

impl CommandContext {
    /// Create a new CommandContext from CLI args and file config
    pub fn new(
        ollama_url: Option<String>,
        model: Option<String>,
        verbose: u8,
        file_config: AgentFileConfig,
    ) -> Self {
        // Resolve with priority: CLI/env > config file > defaults
        let ollama_url = ollama_url.unwrap_or_else(|| file_config.llm.url.clone());
        let model = model.unwrap_or_else(|| file_config.llm.model.clone());

        Self {
            ollama_url,
            model,
            verbose,
            file_config,
            #[cfg(feature = "mcp")]
            mcp_pool: OnceCell::new(),
        }
    }

    /// Create an OllamaClient configured with the context's settings
    pub fn llm(&self) -> OllamaClient {
        OllamaClient::new(&self.ollama_url, &self.model)
    }

    /// Check if verbose mode is enabled (any -v flag)
    pub fn is_verbose(&self) -> bool {
        self.verbose >= 1
    }

    /// Check if debug mode is enabled (-vv or higher)
    pub fn is_debug(&self) -> bool {
        self.verbose >= 2
    }

    /// Get MCP pool, loading lazily if not already loaded.
    /// Returns None if no .mcp.json is found.
    #[cfg(feature = "mcp")]
    pub async fn mcp_pool(&self) -> Result<Option<&crate::mcp::McpClientPool>> {
        self.mcp_pool
            .get_or_try_init(|| async { crate::mcp::McpClientPool::load() })
            .await
            .map(|opt| opt.as_ref())
    }

    /// Get MCP pool, returning an error if not available.
    #[cfg(feature = "mcp")]
    pub async fn mcp_pool_required(&self) -> Result<&crate::mcp::McpClientPool> {
        self.mcp_pool()
            .await?
            .ok_or_else(|| anyhow::anyhow!("No .mcp.json found - MCP tools required"))
    }

    /// Resolve system prompt with priority: explicit > config > auto-generated
    pub fn resolve_system_prompt(&self, explicit: Option<String>) -> Option<String> {
        explicit
            .or_else(|| self.file_config.agent.system_prompt.clone())
            .or_else(|| {
                #[cfg(feature = "mcp")]
                {
                    let ctx = crate::context::EnvironmentContext::gather();
                    Some(ctx.to_system_prompt())
                }
                #[cfg(not(feature = "mcp"))]
                None
            })
    }

    /// Parse server filter from comma-separated string
    pub fn parse_server_filter(servers: Option<String>) -> Option<Vec<String>> {
        servers.map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
    }
}
