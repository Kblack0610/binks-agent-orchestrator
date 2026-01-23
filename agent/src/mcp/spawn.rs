//! Spawn-per-call MCP client
//!
//! Provides one-shot operations by spawning a new process for each call.
//! This is the legacy fallback that works everywhere, including server mode.

use anyhow::{Context, Result};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult},
    transport::TokioChildProcess,
    ServiceExt,
};
use serde_json::Value;
use tokio::process::Command;

use crate::config::McpServerConfig;
use super::types::McpTool;

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
