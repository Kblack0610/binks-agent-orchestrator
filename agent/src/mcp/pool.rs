//! MCP Client Pool with tool caching
//!
//! Pool for managing MCP server access with tool caching.
//! This struct is Send-safe and can be used in async contexts that require Send.
//! When the MCP daemon is running, it uses the daemon for all operations.
//! Otherwise, it falls back to spawn-per-call.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use rmcp::model::{CallToolResult, RawContent, RawTextContent};
use serde_json::Value;

use super::model_size::ModelSize;
use super::spawn::McpClient;
use super::types::McpTool;
use crate::config::{McpConfig, McpProfile, McpServerConfig};
use crate::mcps::{is_daemon_running, DaemonClient};

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
    /// Timeout for connecting to daemon socket or spawning MCP process
    connect_timeout: Duration,
    /// Timeout for MCP server startup/initialization
    startup_timeout: Duration,
    /// Timeout for individual tool calls
    tool_timeout: Duration,
}

/// Default connect timeout (daemon socket or spawn connection)
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Default MCP server startup timeout
const DEFAULT_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
/// Default tool call timeout
const DEFAULT_TOOL_TIMEOUT: Duration = Duration::from_secs(60);

impl McpClientPool {
    /// Create a new pool from config (uses default timeouts)
    pub fn new(config: McpConfig) -> Self {
        Self {
            config,
            tools_cache: HashMap::new(),
            daemon_available: None,
            daemon_client: DaemonClient::new()
                .with_connect_timeout(DEFAULT_CONNECT_TIMEOUT)
                .with_read_timeout(DEFAULT_TOOL_TIMEOUT),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            startup_timeout: DEFAULT_STARTUP_TIMEOUT,
            tool_timeout: DEFAULT_TOOL_TIMEOUT,
        }
    }

    /// Create a pool with custom timeouts
    pub fn with_timeouts(
        mut self,
        connect_timeout: Duration,
        startup_timeout: Duration,
        tool_timeout: Duration,
    ) -> Self {
        self.connect_timeout = connect_timeout;
        self.startup_timeout = startup_timeout;
        self.tool_timeout = tool_timeout;
        self.daemon_client = DaemonClient::new()
            .with_connect_timeout(connect_timeout)
            .with_read_timeout(tool_timeout);
        self
    }

    /// Create an empty pool for testing purposes
    #[cfg(test)]
    pub fn empty() -> Self {
        Self::new(McpConfig {
            mcp_servers: HashMap::new(),
        })
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

    /// Get cached daemon availability state (non-async, returns last known value)
    pub fn is_daemon_available(&self) -> bool {
        self.daemon_available.unwrap_or(false)
    }

    /// Force recheck of daemon availability
    pub fn reset_daemon_check(&mut self) {
        self.daemon_available = None;
    }

    /// Get the number of cached tools for a server
    pub fn cached_tool_count(&self, name: &str) -> usize {
        self.tools_cache.get(name).map(|v| v.len()).unwrap_or(0)
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
                .filter(|name| *name != "agent" && self.config.mcp_servers.contains_key(*name))
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
            McpClient::list_tools_with_timeout(name, server_config, self.startup_timeout)
                .await?
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
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult> {
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
            let daemon_result = self
                .daemon_client
                .call_tool(&server_name, tool_name, arguments)
                .await?;

            // Convert daemon result to CallToolResult
            // rmcp types: Content = Annotated<RawContent>, RawContent::Text(RawTextContent)
            let content: Vec<rmcp::model::Content> = daemon_result
                .content
                .into_iter()
                .filter_map(|c| {
                    c.text.map(|t| rmcp::model::Content {
                        raw: RawContent::Text(RawTextContent {
                            text: t,
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

            McpClient::call_tool_with_timeouts(
                &server_name,
                server_config,
                tool_name,
                arguments,
                self.startup_timeout,
                self.tool_timeout,
            )
            .await
        }
    }

    /// Look up which server owns a tool (from the tools cache)
    ///
    /// Returns the server name if the tool is found in the cache.
    /// This is useful for recording per-server metrics after a tool call.
    pub fn server_for_tool(&self, tool_name: &str) -> Option<String> {
        for (server_name, tools) in &self.tools_cache {
            if tools.iter().any(|t| t.name == tool_name) {
                return Some(server_name.clone());
            }
        }
        None
    }

    /// Clear the tools cache
    pub fn clear_cache(&mut self) {
        self.tools_cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Helper to create a minimal MCP config for testing
    fn create_test_config(servers: Vec<(&str, u8)>) -> McpConfig {
        let mut mcp_servers = HashMap::new();
        for (name, tier) in servers {
            mcp_servers.insert(
                name.to_string(),
                McpServerConfig {
                    command: format!("/path/to/{}", name),
                    args: vec![],
                    env: HashMap::new(),
                    tier,
                },
            );
        }
        McpConfig { mcp_servers }
    }

    // ============== server_names Tests ==============

    #[test]
    fn test_server_names_returns_all_configured() {
        let config = create_test_config(vec![("sysinfo", 1), ("kubernetes", 2), ("github", 3)]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"sysinfo".to_string()));
        assert!(names.contains(&"kubernetes".to_string()));
        assert!(names.contains(&"github".to_string()));
    }

    #[test]
    fn test_server_names_excludes_agent() {
        let config = create_test_config(vec![
            ("sysinfo", 1),
            ("agent", 4), // Should be excluded
            ("kubernetes", 2),
        ]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names();
        assert_eq!(names.len(), 2);
        assert!(!names.contains(&"agent".to_string()));
    }

    #[test]
    fn test_server_names_empty_config() {
        let config = create_test_config(vec![]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names();
        assert!(names.is_empty());
    }

    // ============== server_names_for_tier Tests ==============

    #[test]
    fn test_tier_filtering_tier_1_only() {
        let config = create_test_config(vec![
            ("sysinfo", 1),
            ("kubernetes", 2),
            ("github", 3),
            ("extended", 4),
        ]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names_for_tier(1);
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"sysinfo".to_string()));
    }

    #[test]
    fn test_tier_filtering_tier_2() {
        let config = create_test_config(vec![
            ("sysinfo", 1),
            ("kubernetes", 2),
            ("github", 3),
            ("extended", 4),
        ]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names_for_tier(2);
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"sysinfo".to_string()));
        assert!(names.contains(&"kubernetes".to_string()));
    }

    #[test]
    fn test_tier_filtering_tier_3() {
        let config = create_test_config(vec![
            ("sysinfo", 1),
            ("kubernetes", 2),
            ("github", 3),
            ("extended", 4),
        ]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names_for_tier(3);
        assert_eq!(names.len(), 3);
        assert!(!names.contains(&"extended".to_string()));
    }

    #[test]
    fn test_tier_filtering_tier_4_includes_all() {
        let config = create_test_config(vec![
            ("sysinfo", 1),
            ("kubernetes", 2),
            ("github", 3),
            ("extended", 4),
        ]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names_for_tier(4);
        assert_eq!(names.len(), 4);
    }

    #[test]
    fn test_tier_filtering_excludes_agent() {
        let config = create_test_config(vec![
            ("sysinfo", 1),
            ("agent", 1), // Same tier, should still be excluded
        ]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names_for_tier(1);
        assert_eq!(names.len(), 1);
        assert!(!names.contains(&"agent".to_string()));
    }

    #[test]
    fn test_tier_filtering_zero_excludes_all() {
        let config = create_test_config(vec![("sysinfo", 1), ("kubernetes", 2)]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names_for_tier(0);
        assert!(names.is_empty());
    }

    #[test]
    fn test_tier_filtering_max_u8_includes_all() {
        let config = create_test_config(vec![
            ("sysinfo", 1),
            ("kubernetes", 255), // Max tier
        ]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names_for_tier(255);
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_tier_filtering_same_tier_multiple_servers() {
        let config = create_test_config(vec![("sysinfo", 1), ("memory", 1), ("filesystem", 1)]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names_for_tier(1);
        assert_eq!(names.len(), 3);
    }

    // ============== server_names_for_model_size Tests ==============

    #[test]
    fn test_model_size_small_gets_tier_1() {
        let config = create_test_config(vec![("sysinfo", 1), ("kubernetes", 2), ("github", 3)]);
        let pool = McpClientPool::new(config);

        // Small models (≤8B) get tier 1 by default
        let names = pool.server_names_for_model_size(ModelSize::Small);
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"sysinfo".to_string()));
    }

    #[test]
    fn test_model_size_medium_gets_tier_2() {
        let config = create_test_config(vec![("sysinfo", 1), ("kubernetes", 2), ("github", 3)]);
        let pool = McpClientPool::new(config);

        // Medium models (≤32B) get tier 2 by default
        let names = pool.server_names_for_model_size(ModelSize::Medium);
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_model_size_large_gets_tier_3() {
        let config = create_test_config(vec![
            ("sysinfo", 1),
            ("kubernetes", 2),
            ("github", 3),
            ("extended", 4),
        ]);
        let pool = McpClientPool::new(config);

        // Large models (>32B) get tier 3 by default (all except agent-only)
        let names = pool.server_names_for_model_size(ModelSize::Large);
        assert_eq!(names.len(), 3);
        assert!(!names.contains(&"extended".to_string()));
    }

    #[test]
    fn test_model_size_unknown_gets_tier_1() {
        let config = create_test_config(vec![("sysinfo", 1), ("kubernetes", 2), ("github", 3)]);
        let pool = McpClientPool::new(config);

        // Unknown models get tier 1 by default (conservative - treat as small)
        let names = pool.server_names_for_model_size(ModelSize::Unknown);
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"sysinfo".to_string()));
    }

    // ============== server_names_for_profile Tests ==============

    #[test]
    fn test_profile_explicit_servers_list() {
        let config = create_test_config(vec![("sysinfo", 1), ("kubernetes", 2), ("github", 3)]);
        let pool = McpClientPool::new(config);

        let profile = McpProfile {
            max_tier: 4, // Should be ignored when servers is Some
            servers: Some(vec!["sysinfo".to_string(), "github".to_string()]),
        };

        let names = pool.server_names_for_profile(&profile);
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"sysinfo".to_string()));
        assert!(names.contains(&"github".to_string()));
        assert!(!names.contains(&"kubernetes".to_string()));
    }

    #[test]
    fn test_profile_explicit_servers_filters_invalid() {
        let config = create_test_config(vec![("sysinfo", 1), ("kubernetes", 2)]);
        let pool = McpClientPool::new(config);

        let profile = McpProfile {
            max_tier: 4,
            servers: Some(vec![
                "sysinfo".to_string(),
                "nonexistent".to_string(), // Should be filtered out
            ]),
        };

        let names = pool.server_names_for_profile(&profile);
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"sysinfo".to_string()));
    }

    #[test]
    fn test_profile_explicit_servers_excludes_agent() {
        let config = create_test_config(vec![("sysinfo", 1), ("agent", 4)]);
        let pool = McpClientPool::new(config);

        let profile = McpProfile {
            max_tier: 4,
            servers: Some(vec![
                "sysinfo".to_string(),
                "agent".to_string(), // Should be excluded
            ]),
        };

        let names = pool.server_names_for_profile(&profile);
        assert_eq!(names.len(), 1);
        assert!(!names.contains(&"agent".to_string()));
    }

    #[test]
    fn test_profile_tier_based_when_no_servers() {
        let config = create_test_config(vec![("sysinfo", 1), ("kubernetes", 2), ("github", 3)]);
        let pool = McpClientPool::new(config);

        let profile = McpProfile {
            max_tier: 2,
            servers: None, // Use tier-based filtering
        };

        let names = pool.server_names_for_profile(&profile);
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"sysinfo".to_string()));
        assert!(names.contains(&"kubernetes".to_string()));
    }

    #[test]
    fn test_profile_empty_servers_list() {
        let config = create_test_config(vec![("sysinfo", 1), ("kubernetes", 2)]);
        let pool = McpClientPool::new(config);

        let profile = McpProfile {
            max_tier: 4,
            servers: Some(vec![]), // Empty explicit list
        };

        let names = pool.server_names_for_profile(&profile);
        assert!(names.is_empty());
    }

    // ============== get_server_config Tests ==============

    #[test]
    fn test_get_server_config_exists() {
        let config = create_test_config(vec![("sysinfo", 1)]);
        let pool = McpClientPool::new(config);

        let server_config = pool.get_server_config("sysinfo");
        assert!(server_config.is_some());
        assert_eq!(server_config.unwrap().tier, 1);
    }

    #[test]
    fn test_get_server_config_not_found() {
        let config = create_test_config(vec![("sysinfo", 1)]);
        let pool = McpClientPool::new(config);

        let server_config = pool.get_server_config("nonexistent");
        assert!(server_config.is_none());
    }

    // ============== Cache Tests ==============

    #[test]
    fn test_has_cached_tools_false_initially() {
        let config = create_test_config(vec![("sysinfo", 1)]);
        let pool = McpClientPool::new(config);

        assert!(!pool.has_cached_tools("sysinfo"));
    }

    #[test]
    fn test_clear_cache() {
        let config = create_test_config(vec![("sysinfo", 1)]);
        let mut pool = McpClientPool::new(config);

        // Manually add to cache for testing
        pool.tools_cache.insert("sysinfo".to_string(), vec![]);
        assert!(pool.has_cached_tools("sysinfo"));

        pool.clear_cache();
        assert!(!pool.has_cached_tools("sysinfo"));
    }

    // ============== Edge Cases ==============

    #[test]
    fn test_server_names_with_special_characters() {
        let config = create_test_config(vec![
            ("mcp__sysinfo__server", 1),
            ("my-server-name", 2),
            ("server.with.dots", 3),
        ]);
        let pool = McpClientPool::new(config);

        let names = pool.server_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"mcp__sysinfo__server".to_string()));
        assert!(names.contains(&"my-server-name".to_string()));
        assert!(names.contains(&"server.with.dots".to_string()));
    }

    #[test]
    fn test_tier_filtering_preserves_all_tiers() {
        // Test that servers are correctly categorized at each tier boundary
        let config = create_test_config(vec![
            ("tier1a", 1),
            ("tier1b", 1),
            ("tier2", 2),
            ("tier3", 3),
            ("tier4", 4),
        ]);
        let pool = McpClientPool::new(config);

        assert_eq!(pool.server_names_for_tier(1).len(), 2);
        assert_eq!(pool.server_names_for_tier(2).len(), 3);
        assert_eq!(pool.server_names_for_tier(3).len(), 4);
        assert_eq!(pool.server_names_for_tier(4).len(), 5);
    }
}
