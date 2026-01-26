//! Client for communicating with the MCP daemon
//!
//! This module provides a Send-safe client that communicates with the
//! MCP daemon via Unix socket.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use super::protocol::*;

/// Client for communicating with the MCP daemon
#[derive(Clone)]
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    /// Create a new daemon client with the default socket path
    pub fn new() -> Self {
        Self {
            socket_path: default_socket_path(),
        }
    }

    /// Create a client with a custom socket path
    pub fn with_socket_path(socket_path: PathBuf) -> Self {
        Self { socket_path }
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

    /// Send a request to the daemon and get a response
    async fn send_request(&self, request: DaemonRequest) -> Result<DaemonResponse> {
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .context(format!(
                "Failed to connect to daemon at {:?}",
                self.socket_path
            ))?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Send request
        let request_json = serde_json::to_string(&request)?;
        writer.write_all(request_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        // Read response
        let mut line = String::new();
        reader.read_line(&mut line).await?;

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
