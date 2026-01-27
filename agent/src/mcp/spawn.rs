//! Spawn-per-call MCP client
//!
//! Provides one-shot operations by spawning a new process for each call.
//! This is the legacy fallback that works everywhere, including server mode.

use std::time::Duration;

use anyhow::{Context, Result};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult},
    transport::TokioChildProcess,
    ServiceExt,
};
use serde_json::Value;
use tokio::process::Command;

use super::types::McpTool;
use crate::config::McpServerConfig;

/// Default startup timeout for spawning and initializing an MCP server
const DEFAULT_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Default tool call timeout
const DEFAULT_TOOL_TIMEOUT: Duration = Duration::from_secs(60);

/// Single MCP client connection - static methods for one-shot operations
pub struct McpClient;

impl McpClient {
    /// Connect to an MCP server, list its tools, and disconnect
    pub async fn list_tools(name: &str, config: &McpServerConfig) -> Result<Vec<McpTool>> {
        Self::list_tools_with_timeout(name, config, DEFAULT_STARTUP_TIMEOUT).await
    }

    /// Connect to an MCP server, list its tools, and disconnect (with configurable timeout)
    pub async fn list_tools_with_timeout(
        name: &str,
        config: &McpServerConfig,
        startup_timeout: Duration,
    ) -> Result<Vec<McpTool>> {
        tracing::debug!("Connecting to MCP server: {}", name);

        let mut cmd = Command::new(&config.command);
        if !config.args.is_empty() {
            cmd.args(&config.args);
        }
        for (key, value) in &config.env {
            let expanded = shellexpand::env(value).unwrap_or_else(|_| value.clone().into());
            cmd.env(key, expanded.as_ref());
        }

        // Wrap spawn + initialization in startup timeout
        let service = tokio::time::timeout(startup_timeout, async {
            let transport = TokioChildProcess::new(cmd)?;
            let svc = ().serve(transport).await?;
            Ok::<_, anyhow::Error>(svc)
        })
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "MCP server '{}' startup timed out after {:?}",
                name,
                startup_timeout
            )
        })??;

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
        Self::call_tool_with_timeouts(
            name,
            config,
            tool_name,
            arguments,
            DEFAULT_STARTUP_TIMEOUT,
            DEFAULT_TOOL_TIMEOUT,
        )
        .await
    }

    /// Connect to an MCP server and call a tool (with configurable timeouts)
    pub async fn call_tool_with_timeouts(
        name: &str,
        config: &McpServerConfig,
        tool_name: &str,
        arguments: Option<Value>,
        startup_timeout: Duration,
        tool_timeout: Duration,
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

        // Wrap spawn + initialization in startup timeout
        let service = tokio::time::timeout(startup_timeout, async {
            let transport = TokioChildProcess::new(cmd)?;
            let svc = ().serve(transport).await?;
            Ok::<_, anyhow::Error>(svc)
        })
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "MCP server '{}' startup timed out after {:?}",
                name,
                startup_timeout
            )
        })??;

        let args = arguments.and_then(|v| v.as_object().cloned());

        // Wrap tool call in tool timeout
        let result = tokio::time::timeout(
            tool_timeout,
            service.call_tool(CallToolRequestParam {
                name: tool_name.to_string().into(),
                arguments: args,
                task: None,
            }),
        )
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "Tool '{}' on server '{}' timed out after {:?}",
                tool_name,
                name,
                tool_timeout
            )
        })?
        .context("Failed to call tool")?;

        service.cancel().await?;
        Ok(result)
    }
}
