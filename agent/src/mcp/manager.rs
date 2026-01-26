//! MCP Connection Manager
//!
//! Channel-based manager for persistent connections.
//! This manager owns MCP connections and is NOT Send. It communicates
//! with the rest of the application via channels.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use rmcp::model::CallToolResult;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use super::spawn::McpClient;
use super::types::McpTool;
use crate::config::{McpConfig, McpServerConfig};

// =============================================================================
// Request Types
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

// =============================================================================
// Manager Handle (Send + Sync safe)
// =============================================================================

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

// =============================================================================
// Managed Connection (internal)
// =============================================================================

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

    async fn call_tool(
        &mut self,
        name: &str,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult> {
        self.touch();
        McpClient::call_tool(name, &self.config, tool_name, arguments).await
    }
}

// =============================================================================
// Connection Manager
// =============================================================================

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
