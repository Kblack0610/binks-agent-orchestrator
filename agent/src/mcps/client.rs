//! Client for communicating with the MCP daemon
//!
//! This module provides a Send-safe client that communicates with the
//! MCP daemon via Unix socket.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use super::protocol::*;

/// Client for communicating with the MCP daemon
#[derive(Clone)]
pub struct DaemonClient {
    socket_path: PathBuf,
    /// Timeout for connecting to the daemon socket
    connect_timeout: Duration,
    /// Timeout for reading a response from the daemon
    read_timeout: Duration,
}

impl DaemonClient {
    /// Create a new daemon client with the default socket path and timeouts
    pub fn new() -> Self {
        Self {
            socket_path: default_socket_path(),
            connect_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(60),
        }
    }

    /// Create a client with a custom socket path
    pub fn with_socket_path(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            connect_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(60),
        }
    }

    /// Set the connect timeout
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the read timeout (used for tool call responses)
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Check if the daemon is running
    pub async fn is_running(&self) -> bool {
        self.ping().await.is_ok()
    }

    /// Ping the daemon
    pub async fn ping(&self) -> Result<()> {
        let response = self.send_request(DaemonRequest::Ping).await?;
        match response {
            DaemonResponse::Pong => Ok(()),
            DaemonResponse::Error { message } => anyhow::bail!("{}", message),
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// Get status of all servers
    pub async fn status(&self) -> Result<Vec<ServerStatus>> {
        let response = self.send_request(DaemonRequest::Status).await?;
        match response {
            DaemonResponse::Status { servers } => Ok(servers),
            DaemonResponse::Error { message } => anyhow::bail!("{}", message),
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// List tools from a specific server
    pub async fn list_tools(&self, server: &str) -> Result<Vec<ToolInfo>> {
        let response = self
            .send_request(DaemonRequest::ListTools {
                server: server.to_string(),
            })
            .await?;
        match response {
            DaemonResponse::Tools { tools } => Ok(tools),
            DaemonResponse::Error { message } => anyhow::bail!("{}", message),
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// List all tools from all servers
    pub async fn list_all_tools(&self) -> Result<Vec<ToolInfo>> {
        let response = self.send_request(DaemonRequest::ListAllTools).await?;
        match response {
            DaemonResponse::Tools { tools } => Ok(tools),
            DaemonResponse::Error { message } => anyhow::bail!("{}", message),
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// Call a tool on a specific server
    pub async fn call_tool(
        &self,
        server: &str,
        tool: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<ToolCallResult> {
        let response = self
            .send_request(DaemonRequest::CallTool {
                server: server.to_string(),
                tool: tool.to_string(),
                arguments,
            })
            .await?;
        match response {
            DaemonResponse::ToolResult { result } => Ok(result),
            DaemonResponse::Error { message } => anyhow::bail!("{}", message),
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// Refresh a specific server
    pub async fn refresh_server(&self, server: &str) -> Result<()> {
        let response = self
            .send_request(DaemonRequest::RefreshServer {
                server: server.to_string(),
            })
            .await?;
        match response {
            DaemonResponse::Ok => Ok(()),
            DaemonResponse::Error { message } => anyhow::bail!("{}", message),
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// Refresh all servers
    pub async fn refresh_all(&self) -> Result<()> {
        let response = self.send_request(DaemonRequest::RefreshAll).await?;
        match response {
            DaemonResponse::Ok => Ok(()),
            DaemonResponse::Error { message } => anyhow::bail!("{}", message),
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// Request daemon shutdown
    pub async fn shutdown(&self) -> Result<()> {
        let response = self.send_request(DaemonRequest::Shutdown).await?;
        match response {
            DaemonResponse::Ok => Ok(()),
            DaemonResponse::Error { message } => anyhow::bail!("{}", message),
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// Connect to the daemon socket with timeout and single retry
    async fn connect_with_retry(&self) -> Result<UnixStream> {
        // First attempt
        match tokio::time::timeout(self.connect_timeout, UnixStream::connect(&self.socket_path))
            .await
        {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(e)) => {
                // Connection error — retry once after a short delay
                // (daemon may be mid-restart)
                tracing::warn!(
                    socket = ?self.socket_path,
                    error = %e,
                    "Daemon connect failed, retrying in 500ms"
                );
            }
            Err(_) => {
                tracing::warn!(
                    socket = ?self.socket_path,
                    timeout_ms = self.connect_timeout.as_millis() as u64,
                    "Daemon connect timed out, retrying in 500ms"
                );
            }
        }

        // Single retry after 500ms
        tokio::time::sleep(Duration::from_millis(500)).await;

        let stream =
            tokio::time::timeout(self.connect_timeout, UnixStream::connect(&self.socket_path))
                .await
                .map_err(|_| {
                    anyhow::anyhow!(
                        "Daemon connect timed out after {:?} (retry) at {:?}",
                        self.connect_timeout,
                        self.socket_path
                    )
                })?
                .context(format!(
                    "Failed to connect to daemon at {:?} (retry)",
                    self.socket_path
                ))?;

        tracing::info!("Daemon connect succeeded on retry");
        Ok(stream)
    }

    /// Send a request to the daemon and get a response
    async fn send_request(&self, request: DaemonRequest) -> Result<DaemonResponse> {
        let stream = self.connect_with_retry().await?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Send request
        let request_json = serde_json::to_string(&request)?;
        writer.write_all(request_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        // Read response with timeout
        let mut line = String::new();
        tokio::time::timeout(self.read_timeout, reader.read_line(&mut line))
            .await
            .map_err(|_| {
                anyhow::anyhow!("Daemon response timed out after {:?}", self.read_timeout)
            })?
            .context("Failed to read daemon response")?;

        let response: DaemonResponse =
            serde_json::from_str(&line).context("Failed to parse daemon response")?;

        Ok(response)
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self::new()
    }
}
