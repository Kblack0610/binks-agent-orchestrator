//! MCP Client implementation
//!
//! Supports three modes:
//! 1. Daemon mode (preferred when daemon is running) - persistent connections via Unix socket
//! 2. Spawn-per-call (fallback, works everywhere including server mode)
//! 3. Connection manager for in-process persistence (Phase 2 internal use)
//!
//! Tool discovery results are cached to avoid repeated server queries.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult, RawContent, RawTextContent},
    transport::TokioChildProcess,
    ServiceExt,
};
use serde_json::Value;
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

use crate::config::{McpConfig, McpServerConfig};
use crate::mcps::{DaemonClient, is_daemon_running};

/// A tool from an MCP server
#[derive(Debug, Clone)]
pub struct McpTool {
    /// Server this tool belongs to
    pub server: String,
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: Option<String>,
    /// Input schema (JSON)
    pub input_schema: Option<Value>,
}

// =============================================================================
// Spawn-per-call client (legacy, always works)
// =============================================================================

/// Single MCP client connection - static methods for one-shot operations
pub struct McpClient;

impl McpClient {
    /// Connect to an MCP server, list its tools, and disconnect
    pub async fn list_tools(name: &str, config: &McpServerConfig) -> Result<Vec<McpTool>> {
        tracing::debug!("Connecting to MCP server: {}", name);

        let mut cmd = Command::new(&config.command);
        if !config.args.is_empty() {
            cmd.args(&config.args);
        }
        for (key, value) in &config.env {
            let expanded = shellexpand::env(value).unwrap_or_else(|_| value.clone().into());
            cmd.env(key, expanded.as_ref());
        }

        let transport = TokioChildProcess::new(cmd)?;
        let service = ().serve(transport).await?;

        let response = service
            .list_tools(Default::default())
            .await
            .context("Failed to list tools")?;

        let tools = response
            .tools
            .into_iter()
            .map(|t| McpTool {
                server: name.to_string(),
                name: t.name.to_string(),
                description: t.description.map(|d| d.to_string()),
                input_schema: Some(serde_json::to_value(&t.input_schema).unwrap_or_default()),
            })
            .collect();

        service.cancel().await?;
        Ok(tools)
    }

    /// Connect to an MCP server and call a tool
    pub async fn call_tool(
        name: &str,
        config: &McpServerConfig,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult> {
        tracing::debug!("Connecting to MCP server: {} to call {}", name, tool_name);

        let mut cmd = Command::new(&config.command);
        if !config.args.is_empty() {
            cmd.args(&config.args);
        }
        for (key, value) in &config.env {
            let expanded = shellexpand::env(value).unwrap_or_else(|_| value.clone().into());
            cmd.env(key, expanded.as_ref());
        }

        let transport = TokioChildProcess::new(cmd)?;
        let service = ().serve(transport).await?;

        let args = arguments.and_then(|v| v.as_object().cloned());
        let result = service
            .call_tool(CallToolRequestParam {
                name: tool_name.to_string().into(),
                arguments: args,
                task: None,
            })
            .await
            .context("Failed to call tool")?;

        service.cancel().await?;
        Ok(result)
    }
}

// =============================================================================
// Pool with tool caching (Send-safe)
// =============================================================================

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

// =============================================================================
// Connection Manager (for persistent connections, Phase 2)
// =============================================================================

/// Request types for the connection manager
#[derive(Debug)]
pub enum McpRequest {
    ListTools {
        server: String,
        reply: oneshot::Sender<Result<Vec<McpTool>>>,
    },
    CallTool {
        server: String,
        tool_name: String,
        arguments: Option<Value>,
        reply: oneshot::Sender<Result<CallToolResult>>,
    },
    GetStatus {
        reply: oneshot::Sender<Vec<(String, bool, Option<Duration>)>>,
    },
    Shutdown,
}

/// Handle for communicating with the connection manager
///
/// This struct is Send + Sync safe and can be cloned.
#[derive(Clone)]
pub struct McpManagerHandle {
    sender: mpsc::Sender<McpRequest>,
}

impl McpManagerHandle {
    /// List tools from a specific server
    pub async fn list_tools(&self, server: &str) -> Result<Vec<McpTool>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(McpRequest::ListTools {
                server: server.to_string(),
                reply: tx,
            })
            .await
            .context("Connection manager not running")?;
        rx.await.context("Failed to receive response")?
    }

    /// Call a tool
    pub async fn call_tool(
        &self,
        server: &str,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(McpRequest::CallTool {
                server: server.to_string(),
                tool_name: tool_name.to_string(),
                arguments,
                reply: tx,
            })
            .await
            .context("Connection manager not running")?;
        rx.await.context("Failed to receive response")?
    }

    /// Get connection status for all servers
    pub async fn get_status(&self) -> Result<Vec<(String, bool, Option<Duration>)>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(McpRequest::GetStatus { reply: tx })
            .await
            .context("Connection manager not running")?;
        rx.await.context("Failed to receive response")
    }

    /// Request shutdown of the connection manager
    pub async fn shutdown(&self) -> Result<()> {
        self.sender
            .send(McpRequest::Shutdown)
            .await
            .context("Connection manager not running")?;
        Ok(())
    }
}

/// Connection state for a single MCP server
struct ManagedConnection {
    config: McpServerConfig,
    last_used: Instant,
    tools_cache: Option<Vec<McpTool>>,
}

impl ManagedConnection {
    fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            last_used: Instant::now(),
            tools_cache: None,
        }
    }

    fn touch(&mut self) {
        self.last_used = Instant::now();
    }

    fn idle_time(&self) -> Duration {
        self.last_used.elapsed()
    }

    async fn list_tools(&mut self, name: &str) -> Result<Vec<McpTool>> {
        // Check cache
        if let Some(ref tools) = self.tools_cache {
            return Ok(tools.clone());
        }

        // Fetch (spawn-per-call for now, persistent in Phase 2)
        let tools = McpClient::list_tools(name, &self.config).await?;
        self.tools_cache = Some(tools.clone());
        self.touch();
        Ok(tools)
    }

    async fn call_tool(&mut self, name: &str, tool_name: &str, arguments: Option<Value>) -> Result<CallToolResult> {
        self.touch();
        McpClient::call_tool(name, &self.config, tool_name, arguments).await
    }
}

/// Connection manager that runs in its own task
///
/// This manager owns MCP connections and is NOT Send. It communicates
/// with the rest of the application via channels.
pub struct McpConnectionManager {
    connections: HashMap<String, ManagedConnection>,
}

impl McpConnectionManager {
    /// Create a new connection manager from config
    pub fn new(config: McpConfig) -> Self {
        let connections: HashMap<String, ManagedConnection> = config
            .mcp_servers
            .into_iter()
            .filter(|(name, _)| name != "agent")
            .map(|(name, cfg)| (name, ManagedConnection::new(cfg)))
            .collect();

        Self { connections }
    }

    /// Spawn the manager as a background task and return a handle
    pub fn spawn(config: McpConfig) -> McpManagerHandle {
        let (tx, rx) = mpsc::channel(32);
        let manager = Self::new(config);

        tokio::spawn(async move {
            manager.run(rx).await;
        });

        McpManagerHandle { sender: tx }
    }

    /// Run the connection manager event loop
    async fn run(mut self, mut rx: mpsc::Receiver<McpRequest>) {
        tracing::info!("MCP connection manager started");

        while let Some(request) = rx.recv().await {
            match request {
                McpRequest::ListTools { server, reply } => {
                    let result = if let Some(conn) = self.connections.get_mut(&server) {
                        conn.list_tools(&server).await
                    } else {
                        Err(anyhow::anyhow!("Server '{}' not configured", server))
                    };
                    let _ = reply.send(result);
                }

                McpRequest::CallTool {
                    server,
                    tool_name,
                    arguments,
                    reply,
                } => {
                    let result = if let Some(conn) = self.connections.get_mut(&server) {
                        conn.call_tool(&server, &tool_name, arguments).await
                    } else {
                        Err(anyhow::anyhow!("Server '{}' not configured", server))
                    };
                    let _ = reply.send(result);
                }

                McpRequest::GetStatus { reply } => {
                    let status: Vec<_> = self
                        .connections
                        .iter()
                        .map(|(name, conn)| {
                            let has_cache = conn.tools_cache.is_some();
                            (name.clone(), has_cache, Some(conn.idle_time()))
                        })
                        .collect();
                    let _ = reply.send(status);
                }

                McpRequest::Shutdown => {
                    tracing::info!("MCP connection manager shutting down");
                    break;
                }
            }
        }

        tracing::info!("MCP connection manager stopped");
    }
}
