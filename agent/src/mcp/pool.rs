//! MCP Client Pool with tool caching
//!
//! Pool for managing MCP server access with tool caching.
//! This struct is Send-safe and can be used in async contexts that require Send.
//! When the MCP daemon is running, it uses the daemon for all operations.
//! Otherwise, it falls back to spawn-per-call.

use std::collections::HashMap;

use anyhow::{Context, Result};
use rmcp::model::{CallToolResult, RawContent, RawTextContent};
use serde_json::Value;

use crate::config::{McpConfig, McpProfile, McpServerConfig};
use crate::mcps::{DaemonClient, is_daemon_running};
use super::model_size::ModelSize;
use super::spawn::McpClient;
use super::types::McpTool;

/// Pool for managing MCP server access with tool caching
///
/// This struct is Send-safe and can be used in async contexts that require Send.
/// When the MCP daemon is running, it uses the daemon for all operations.
/// Otherwise, it falls back to spawn-per-call.
pub struct McpClientPool {
    config: McpConfig,
    /// Cache of tools per server
    tools_cache: HashMap<String, Vec<McpTool>>,
    /// Cached daemon running status (refreshed periodically)
    daemon_available: Option<bool>,
    /// Daemon client for when daemon is running
    daemon_client: DaemonClient,
}

impl McpClientPool {
    /// Create a new pool from config
    pub fn new(config: McpConfig) -> Self {
        Self {
            config,
            tools_cache: HashMap::new(),
            daemon_available: None,
            daemon_client: DaemonClient::new(),
        }
    }

    /// Check if daemon is available (with caching)
    async fn check_daemon(&mut self) -> bool {
        if self.daemon_available.is_none() {
            let is_running = is_daemon_running().await;
            self.daemon_available = Some(is_running);
            if is_running {
                tracing::info!("MCP daemon detected - using persistent connections");
            }
        }
        self.daemon_available.unwrap_or(false)
    }

    /// Force recheck of daemon availability
    pub fn reset_daemon_check(&mut self) {
        self.daemon_available = None;
    }

    /// Load pool from .mcp.json in current directory tree
    pub fn load() -> Result<Option<Self>> {
        match McpConfig::load()? {
            Some(config) => Ok(Some(Self::new(config))),
            None => Ok(None),
        }
    }

    /// Get list of configured server names (excludes "agent" to prevent recursion)
    pub fn server_names(&self) -> Vec<String> {
        self.config
            .mcp_servers
            .keys()
            .filter(|name| *name != "agent")
            .cloned()
            .collect()
    }

    /// Get server names filtered by maximum tier level
    ///
    /// Returns servers with tier <= max_tier, excluding "agent"
    pub fn server_names_for_tier(&self, max_tier: u8) -> Vec<String> {
        self.config
            .mcp_servers
            .iter()
            .filter(|(name, config)| *name != "agent" && config.tier <= max_tier)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get server names appropriate for a model size class
    ///
    /// Uses the default tier mapping for each size class
    pub fn server_names_for_model_size(&self, size: ModelSize) -> Vec<String> {
        self.server_names_for_tier(size.default_max_tier())
    }

    /// Get server names based on an MCP profile configuration
    ///
    /// If the profile has an explicit servers list, use that.
    /// Otherwise, filter by the profile's max_tier.
    pub fn server_names_for_profile(&self, profile: &McpProfile) -> Vec<String> {
        if let Some(ref servers) = profile.servers {
            // Explicit server list - validate against config and filter out "agent"
            servers
                .iter()
                .filter(|name| {
                    *name != "agent" && self.config.mcp_servers.contains_key(*name)
                })
                .cloned()
                .collect()
        } else {
            // Use tier-based filtering
            self.server_names_for_tier(profile.max_tier)
        }
    }

    /// Get the server config by name
    pub fn get_server_config(&self, name: &str) -> Option<&McpServerConfig> {
        self.config.mcp_servers.get(name)
    }

    /// Check if tools are cached for a server
    pub fn has_cached_tools(&self, name: &str) -> bool {
        self.tools_cache.contains_key(name)
    }

    /// List tools from a specific server (with caching)
    pub async fn list_tools_from(&mut self, name: &str) -> Result<Vec<McpTool>> {
        // Check cache first
        if let Some(tools) = self.tools_cache.get(name) {
            return Ok(tools.clone());
        }

        let tools = if self.check_daemon().await {
            // Use daemon for persistent connection
            let daemon_tools = self.daemon_client.list_tools(name).await?;
            daemon_tools
                .into_iter()
                .map(|t| McpTool {
                    server: t.server,
                    name: t.name,
                    description: t.description,
                    input_schema: t.input_schema,
                })
                .collect()
        } else {
            // Fallback: Get server config and spawn
            let server_config = self
                .config
                .mcp_servers
                .get(name)
                .context(format!("MCP server '{}' not found in config", name))?;
            McpClient::list_tools(name, server_config).await?
        };

        // Cache the result
        self.tools_cache.insert(name.to_string(), tools.clone());

        tracing::info!("Server '{}': {} tools (cached)", name, tools.len());
        Ok(tools)
    }

    /// List all tools from all configured servers
    pub async fn list_all_tools(&mut self) -> Result<Vec<McpTool>> {
        let mut all_tools = Vec::new();

        for name in self.server_names() {
            match self.list_tools_from(&name).await {
                Ok(tools) => {
                    all_tools.extend(tools);
                }
                Err(e) => {
                    tracing::warn!("Failed to list tools from '{}': {}", name, e);
                }
            }
        }

        Ok(all_tools)
    }

    /// Call a tool by name (uses daemon if available, otherwise spawn-per-call)
    pub async fn call_tool(&mut self, tool_name: &str, arguments: Option<Value>) -> Result<CallToolResult> {
        // Find which server has this tool
        let server_name = {
            let mut found = None;
            for name in self.server_names() {
                match self.list_tools_from(&name).await {
                    Ok(tools) => {
                        if tools.iter().any(|t| t.name == tool_name) {
                            found = Some(name);
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to check tools from '{}': {}", name, e);
                    }
                }
            }
            found.context(format!("Tool '{}' not found in any MCP server", tool_name))?
        };

        if self.check_daemon().await {
            // Use daemon for persistent connection
            let daemon_result = self.daemon_client
                .call_tool(&server_name, tool_name, arguments)
                .await?;

            // Convert daemon result to CallToolResult
            // rmcp types: Content = Annotated<RawContent>, RawContent::Text(RawTextContent)
            let content: Vec<rmcp::model::Content> = daemon_result.content
                .into_iter()
                .filter_map(|c| {
                    c.text.map(|t| rmcp::model::Content {
                        raw: RawContent::Text(RawTextContent {
                            text: t.into(),
                            meta: Default::default(),
                        }),
                        annotations: None,
                    })
                })
                .collect();

            Ok(CallToolResult {
                content,
                is_error: Some(daemon_result.is_error),
                meta: Default::default(),
                structured_content: None,
            })
        } else {
            // Fallback: Get server config and spawn
            let server_config = self
                .config
                .mcp_servers
                .get(&server_name)
                .context(format!("MCP server '{}' not found", server_name))?;

            McpClient::call_tool(&server_name, server_config, tool_name, arguments).await
        }
    }

    /// Clear the tools cache
    pub fn clear_cache(&mut self) {
        self.tools_cache.clear();
    }
}
