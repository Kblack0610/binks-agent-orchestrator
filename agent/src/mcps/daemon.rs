//! MCP Daemon - Background supervisor for MCP servers
//!
//! The daemon runs as a background process and manages MCP server lifecycles.
//! It communicates with agents via Unix socket, solving the Send/Sync problem
//! by keeping all non-Send MCP connections in a single process.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult},
    service::RunningService,
    transport::TokioChildProcess,
    RoleClient, ServiceExt,
};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::protocol::*;
use crate::config::{McpConfig, McpServerConfig};

/// Idle timeout before stopping an MCP server (5 minutes)
const IDLE_TIMEOUT: Duration = Duration::from_secs(300);

/// Health check interval
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// Managed MCP server instance
struct ManagedServer {
    name: String,
    config: McpServerConfig,
    state: ServerState,
    service: Option<RunningService<RoleClient, ()>>,
    tools_cache: Option<Vec<ToolInfo>>,
    started_at: Option<Instant>,
    last_used: Option<Instant>,
}

impl ManagedServer {
    fn new(name: String, config: McpServerConfig) -> Self {
        Self {
            name,
            config,
            state: ServerState::Idle,
            service: None,
            tools_cache: None,
            started_at: None,
            last_used: None,
        }
    }

    /// Start the MCP server process
    async fn start(&mut self) -> Result<()> {
        if self.state == ServerState::Running {
            return Ok(());
        }

        self.state = ServerState::Starting;
        tracing::info!("Starting MCP server: {}", self.name);

        let mut cmd = Command::new(&self.config.command);
        if !self.config.args.is_empty() {
            cmd.args(&self.config.args);
        }
        for (key, value) in &self.config.env {
            let expanded = shellexpand::env(value).unwrap_or_else(|_| value.clone().into());
            cmd.env(key, expanded.as_ref());
        }

        let transport = TokioChildProcess::new(cmd)
            .context(format!("Failed to spawn MCP server: {}", self.name))?;

        let service = ()
            .serve(transport)
            .await
            .context(format!("Failed to initialize MCP server: {}", self.name))?;

        self.service = Some(service);
        self.state = ServerState::Running;
        self.started_at = Some(Instant::now());
        self.last_used = Some(Instant::now());

        tracing::info!("MCP server started: {}", self.name);
        Ok(())
    }

    /// Stop the MCP server
    async fn stop(&mut self) -> Result<()> {
        if let Some(service) = self.service.take() {
            tracing::info!("Stopping MCP server: {}", self.name);
            if let Err(e) = service.cancel().await {
                tracing::warn!("Error canceling MCP server {}: {}", self.name, e);
            }
        }
        self.state = ServerState::Stopped;
        self.tools_cache = None;
        Ok(())
    }

    /// List tools from this server
    async fn list_tools(&mut self) -> Result<Vec<ToolInfo>> {
        // Return cached if available
        if let Some(ref tools) = self.tools_cache {
            self.last_used = Some(Instant::now());
            return Ok(tools.clone());
        }

        // Ensure server is running
        if self.state != ServerState::Running {
            self.start().await?;
        }

        let service = self.service.as_ref().context("Service not available")?;

        let response = service
            .list_tools(Default::default())
            .await
            .context("Failed to list tools")?;

        let tools: Vec<ToolInfo> = response
            .tools
            .into_iter()
            .map(|t| ToolInfo {
                server: self.name.clone(),
                name: t.name.to_string(),
                description: t.description.map(|d| d.to_string()),
                input_schema: Some(serde_json::to_value(&t.input_schema).unwrap_or_default()),
            })
            .collect();

        self.tools_cache = Some(tools.clone());
        self.last_used = Some(Instant::now());

        tracing::info!("Server '{}': {} tools discovered", self.name, tools.len());
        Ok(tools)
    }

    /// Call a tool on this server
    async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult> {
        // Ensure server is running
        if self.state != ServerState::Running {
            self.start().await?;
        }

        let service = self.service.as_ref().context("Service not available")?;

        let args = arguments.and_then(|v| v.as_object().cloned());
        let result = service
            .call_tool(CallToolRequestParam {
                name: tool_name.to_string().into(),
                arguments: args,
                task: None,
            })
            .await
            .context("Failed to call tool")?;

        self.last_used = Some(Instant::now());
        Ok(result)
    }

    /// Get server status
    fn status(&self) -> ServerStatus {
        ServerStatus {
            name: self.name.clone(),
            state: self.state,
            tool_count: self.tools_cache.as_ref().map(|t| t.len()).unwrap_or(0),
            last_used_secs: self.last_used.map(|t| t.elapsed().as_secs()),
            uptime_secs: self.started_at.map(|t| t.elapsed().as_secs()),
        }
    }

    /// Check if server should be stopped due to idle timeout
    fn is_idle_expired(&self) -> bool {
        if self.state != ServerState::Running {
            return false;
        }
        self.last_used
            .map(|t| t.elapsed() > IDLE_TIMEOUT)
            .unwrap_or(false)
    }
}

/// The MCP Daemon - manages all MCP servers
pub struct McpDaemon {
    servers: HashMap<String, ManagedServer>,
    socket_path: PathBuf,
}

impl McpDaemon {
    /// Create a new daemon from MCP config
    pub fn new(config: McpConfig, socket_path: PathBuf) -> Self {
        let servers: HashMap<String, ManagedServer> = config
            .mcp_servers
            .into_iter()
            .filter(|(name, _)| name != "agent") // Skip recursive agent
            .map(|(name, cfg)| (name.clone(), ManagedServer::new(name, cfg)))
            .collect();

        Self {
            servers,
            socket_path,
        }
    }

    /// Run the daemon - listens on Unix socket for requests
    pub async fn run(mut self) -> Result<()> {
        // Ensure socket directory exists
        if let Some(parent) = self.socket_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Remove existing socket file
        let _ = tokio::fs::remove_file(&self.socket_path).await;

        let listener = UnixListener::bind(&self.socket_path)
            .context(format!("Failed to bind to socket: {:?}", self.socket_path))?;

        tracing::info!("MCP daemon listening on {:?}", self.socket_path);

        // Channel for internal commands (from health check task)
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<DaemonRequest>(32);

        // Spawn health check task
        let health_tx = cmd_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(HEALTH_CHECK_INTERVAL);
            loop {
                interval.tick().await;
                // Send status request to trigger idle cleanup
                let _ = health_tx.send(DaemonRequest::Status).await;
            }
        });

        loop {
            tokio::select! {
                // Handle socket connections
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, _)) => {
                            if let Err(e) = self.handle_connection(stream).await {
                                tracing::warn!("Connection error: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Accept error: {}", e);
                        }
                    }
                }
                // Handle internal commands
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        DaemonRequest::Status => {
                            // Check for idle servers
                            self.cleanup_idle_servers().await;
                        }
                        DaemonRequest::Shutdown => {
                            tracing::info!("Shutdown requested");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Cleanup
        self.shutdown().await?;
        let _ = tokio::fs::remove_file(&self.socket_path).await;

        tracing::info!("MCP daemon stopped");
        Ok(())
    }

    /// Handle a single client connection
    async fn handle_connection(&mut self, stream: UnixStream) -> Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        // Read request (one JSON line)
        reader.read_line(&mut line).await?;

        if line.is_empty() {
            return Ok(());
        }

        let request: DaemonRequest =
            serde_json::from_str(&line).context("Failed to parse request")?;

        let response = self.handle_request(request).await;

        // Write response
        let response_json = serde_json::to_string(&response)?;
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        Ok(())
    }

    /// Process a daemon request
    async fn handle_request(&mut self, request: DaemonRequest) -> DaemonResponse {
        match request {
            DaemonRequest::Ping => DaemonResponse::Pong,

            DaemonRequest::Status => {
                let servers: Vec<ServerStatus> =
                    self.servers.values().map(|s| s.status()).collect();
                DaemonResponse::Status { servers }
            }

            DaemonRequest::ListTools { server } => match self.servers.get_mut(&server) {
                Some(s) => match s.list_tools().await {
                    Ok(tools) => DaemonResponse::Tools { tools },
                    Err(e) => DaemonResponse::Error {
                        message: e.to_string(),
                    },
                },
                None => DaemonResponse::Error {
                    message: format!("Server '{}' not found", server),
                },
            },

            DaemonRequest::ListAllTools => {
                let mut all_tools = Vec::new();
                let server_names: Vec<String> = self.servers.keys().cloned().collect();

                for name in server_names {
                    if let Some(server) = self.servers.get_mut(&name) {
                        match server.list_tools().await {
                            Ok(tools) => all_tools.extend(tools),
                            Err(e) => {
                                tracing::warn!("Failed to list tools from '{}': {}", name, e);
                            }
                        }
                    }
                }

                DaemonResponse::Tools { tools: all_tools }
            }

            DaemonRequest::CallTool {
                server,
                tool,
                arguments,
            } => match self.servers.get_mut(&server) {
                Some(s) => match s.call_tool(&tool, arguments).await {
                    Ok(result) => {
                        let content: Vec<ToolContent> = result
                            .content
                            .into_iter()
                            .map(|c| {
                                let text = match &c.raw {
                                    rmcp::model::RawContent::Text(t) => Some(t.text.to_string()),
                                    _ => None,
                                };
                                ToolContent {
                                    content_type: "text".to_string(),
                                    text,
                                }
                            })
                            .collect();

                        DaemonResponse::ToolResult {
                            result: ToolCallResult {
                                content,
                                is_error: result.is_error.unwrap_or(false),
                            },
                        }
                    }
                    Err(e) => DaemonResponse::Error {
                        message: e.to_string(),
                    },
                },
                None => DaemonResponse::Error {
                    message: format!("Server '{}' not found", server),
                },
            },

            DaemonRequest::RefreshServer { server } => match self.servers.get_mut(&server) {
                Some(s) => {
                    let _ = s.stop().await;
                    match s.start().await {
                        Ok(()) => DaemonResponse::Ok,
                        Err(e) => DaemonResponse::Error {
                            message: e.to_string(),
                        },
                    }
                }
                None => DaemonResponse::Error {
                    message: format!("Server '{}' not found", server),
                },
            },

            DaemonRequest::RefreshAll => {
                let server_names: Vec<String> = self.servers.keys().cloned().collect();

                for name in server_names {
                    if let Some(server) = self.servers.get_mut(&name) {
                        let _ = server.stop().await;
                        if let Err(e) = server.start().await {
                            tracing::warn!("Failed to restart '{}': {}", name, e);
                        }
                    }
                }

                DaemonResponse::Ok
            }

            DaemonRequest::Shutdown => {
                // This will be handled in the main loop
                DaemonResponse::Ok
            }
        }
    }

    /// Stop idle servers
    async fn cleanup_idle_servers(&mut self) {
        let idle_names: Vec<String> = self
            .servers
            .iter()
            .filter(|(_, s)| s.is_idle_expired())
            .map(|(n, _)| n.clone())
            .collect();

        for name in idle_names {
            if let Some(server) = self.servers.get_mut(&name) {
                tracing::info!("Stopping idle MCP server: {}", name);
                let _ = server.stop().await;
            }
        }
    }

    /// Shutdown all servers
    async fn shutdown(&mut self) -> Result<()> {
        let server_names: Vec<String> = self.servers.keys().cloned().collect();

        for name in server_names {
            if let Some(server) = self.servers.get_mut(&name) {
                let _ = server.stop().await;
            }
        }

        Ok(())
    }
}

/// Check if a daemon is running by trying to connect
pub async fn is_daemon_running() -> bool {
    let socket_path = default_socket_path();
    ping_daemon(&socket_path).await.is_ok()
}

/// Ping the daemon to check if it's alive
pub async fn ping_daemon(socket_path: &std::path::Path) -> Result<()> {
    let stream = UnixStream::connect(socket_path)
        .await
        .context("Failed to connect to daemon")?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send ping
    let request = serde_json::to_string(&DaemonRequest::Ping)?;
    writer.write_all(request.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    // Read response
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response: DaemonResponse = serde_json::from_str(&line)?;
    match response {
        DaemonResponse::Pong => Ok(()),
        DaemonResponse::Error { message } => anyhow::bail!("Daemon error: {}", message),
        _ => anyhow::bail!("Unexpected response"),
    }
}
