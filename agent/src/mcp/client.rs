//! MCP Client implementation

use std::collections::HashMap;

use anyhow::{Context, Result};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult},
    transport::TokioChildProcess,
    ServiceExt,
};
use serde_json::Value;
use tokio::process::Command;

use crate::config::{McpConfig, McpServerConfig};

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

/// Single MCP client connection - connects, lists tools, and disconnects
/// We don't keep persistent connections for simplicity
pub struct McpClient;

impl McpClient {
    /// Connect to an MCP server, list its tools, and disconnect
    pub async fn list_tools(name: &str, config: &McpServerConfig) -> Result<Vec<McpTool>> {
        tracing::info!("Connecting to MCP server: {}", name);

        // Build the command
        let mut cmd = Command::new(&config.command);

        // Add arguments
        if !config.args.is_empty() {
            cmd.args(&config.args);
        }

        // Add environment variables
        for (key, value) in &config.env {
            // Expand ${HOME} and other env vars
            let expanded = shellexpand::env(value).unwrap_or_else(|_| value.clone().into());
            cmd.env(key, expanded.as_ref());
        }

        // Spawn the MCP server as a child process and connect
        let transport = TokioChildProcess::new(cmd)?;
        let service = ().serve(transport).await?;

        tracing::info!("Connected to MCP server: {}", name);

        // List tools
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

        // Shutdown
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
        tracing::info!("Connecting to MCP server: {} to call {}", name, tool_name);

        // Build the command
        let mut cmd = Command::new(&config.command);

        // Add arguments
        if !config.args.is_empty() {
            cmd.args(&config.args);
        }

        // Add environment variables
        for (key, value) in &config.env {
            let expanded = shellexpand::env(value).unwrap_or_else(|_| value.clone().into());
            cmd.env(key, expanded.as_ref());
        }

        // Spawn and connect
        let transport = TokioChildProcess::new(cmd)?;
        let service = ().serve(transport).await?;

        // Call the tool
        let args = arguments.and_then(|v| v.as_object().cloned());
        let result = service
            .call_tool(CallToolRequestParam {
                name: tool_name.to_string().into(),
                arguments: args,
                task: None,
            })
            .await
            .context("Failed to call tool")?;

        // Shutdown
        service.cancel().await?;

        Ok(result)
    }
}

/// Pool for managing MCP server configurations
pub struct McpClientPool {
    config: McpConfig,
    /// Cache of tools per server
    tools_cache: HashMap<String, Vec<McpTool>>,
}

impl McpClientPool {
    /// Create a new pool from config
    pub fn new(config: McpConfig) -> Self {
        Self {
            config,
            tools_cache: HashMap::new(),
        }
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
            .filter(|name| *name != "agent") // Don't connect to ourselves
            .cloned()
            .collect()
    }

    /// List tools from a specific server
    pub async fn list_tools_from(&mut self, name: &str) -> Result<Vec<McpTool>> {
        // Check cache
        if let Some(tools) = self.tools_cache.get(name) {
            return Ok(tools.clone());
        }

        // Get server config
        let server_config = self
            .config
            .mcp_servers
            .get(name)
            .context(format!("MCP server '{}' not found in config", name))?;

        // List tools
        let tools = McpClient::list_tools(name, server_config).await?;

        // Cache
        self.tools_cache.insert(name.to_string(), tools.clone());

        Ok(tools)
    }

    /// List all tools from all configured servers
    pub async fn list_all_tools(&mut self) -> Result<Vec<McpTool>> {
        let mut all_tools = Vec::new();

        for name in self.server_names() {
            match self.list_tools_from(&name).await {
                Ok(tools) => {
                    tracing::info!("Server '{}': {} tools", name, tools.len());
                    all_tools.extend(tools);
                }
                Err(e) => {
                    tracing::warn!("Failed to list tools from '{}': {}", name, e);
                }
            }
        }

        Ok(all_tools)
    }

    /// Call a tool by name (searches all servers)
    pub async fn call_tool(&mut self, tool_name: &str, arguments: Option<Value>) -> Result<CallToolResult> {
        // Find which server has this tool
        for name in self.server_names() {
            match self.list_tools_from(&name).await {
                Ok(tools) => {
                    if tools.iter().any(|t| t.name == tool_name) {
                        let config = self.config.mcp_servers.get(&name).unwrap();
                        return McpClient::call_tool(&name, config, tool_name, arguments).await;
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to check tools from '{}': {}", name, e);
                }
            }
        }

        anyhow::bail!("Tool '{}' not found in any MCP server", tool_name)
    }
}
