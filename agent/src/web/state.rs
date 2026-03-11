//! Shared application state

use anyhow::Result;
use std::sync::Arc;

use crate::db::Database;
use crate::mcp::McpClientPool;
#[cfg(feature = "orchestrator")]
use crate::orchestrator::EngineConfig;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Database connection
    pub db: Database,
    /// MCP client pool for tool access
    pub mcp_pool: Option<Arc<tokio::sync::Mutex<McpClientPool>>>,
    /// LiteLLM gateway URL
    pub gateway_url: String,
    /// Gateway type (currently `litellm`)
    pub gateway_type: String,
    /// Model name
    pub model: String,
    /// Default system prompt
    pub system_prompt: Option<String>,
}

impl AppState {
    /// Create new app state
    pub fn new(
        db: Database,
        gateway_url: String,
        gateway_type: String,
        model: String,
        system_prompt: Option<String>,
    ) -> Result<Self> {
        // Try to load MCP pool
        let mcp_pool = match McpClientPool::load() {
            Ok(Some(pool)) => {
                tracing::info!("MCP pool loaded with servers: {:?}", pool.server_names());
                Some(Arc::new(tokio::sync::Mutex::new(pool)))
            }
            Ok(None) => {
                tracing::warn!("No .mcp.json found - tools will not be available");
                None
            }
            Err(e) => {
                tracing::error!("Failed to load MCP pool: {}", e);
                None
            }
        };

        Ok(Self {
            db,
            mcp_pool,
            gateway_url,
            gateway_type,
            model,
            system_prompt,
        })
    }

    /// Create an EngineConfig from app state
    #[cfg(feature = "orchestrator")]
    pub fn engine_config(&self, non_interactive: bool) -> EngineConfig {
        EngineConfig {
            ollama_url: self.gateway_url.clone(),
            default_model: self.model.clone(),
            non_interactive,
            verbose: false,
            custom_workflows_dir: None,
            record_runs: true, // Enable run recording for web
            db_path: None,     // Use default path
            agent_config: Default::default(),
        }
    }
}
