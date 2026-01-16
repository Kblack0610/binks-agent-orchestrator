//! MCP Server implementation
//!
//! This module exposes the agent as an MCP server, allowing other tools
//! to interact with the LLM and agent capabilities.
//!
//! Tools exposed:
//! - `chat` - Send a message to the LLM (simple chat, no tools)
//! - `agent_chat` - Send a message through the tool-using agent
//! - `list_tools` - List available MCP tools from connected servers

use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent::Agent;
use crate::llm::{Llm, OllamaClient};
use crate::mcp::McpClientPool;

/// Configuration for the agent MCP server
#[derive(Clone)]
pub struct ServerConfig {
    pub ollama_url: String,
    pub model: String,
    pub system_prompt: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.1:8b".to_string(),
            system_prompt: None,
        }
    }
}

/// The Agent MCP Server
///
/// Exposes LLM chat and agent capabilities as MCP tools
pub struct AgentMcpServer {
    tool_router: ToolRouter<Self>,
    config: ServerConfig,
    /// Lazy-initialized agent (needs MCP pool which is async)
    agent: Arc<Mutex<Option<Agent>>>,
    /// Simple LLM client for non-agent chat
    llm: OllamaClient,
}

// ============================================================================
// Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChatParams {
    #[schemars(description = "The message to send to the LLM")]
    pub message: String,
    #[schemars(description = "Optional system prompt to set context")]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AgentChatParams {
    #[schemars(description = "The message to send to the agent")]
    pub message: String,
    #[schemars(description = "Optional system prompt for the agent")]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListToolsParams {
    #[schemars(description = "Optional server name to filter tools")]
    pub server: Option<String>,
}

// ============================================================================
// Tool Router Implementation
// ============================================================================

#[tool_router]
impl AgentMcpServer {
    pub fn new(config: ServerConfig) -> Self {
        let llm = OllamaClient::new(&config.ollama_url, &config.model);
        Self {
            tool_router: Self::tool_router(),
            config,
            agent: Arc::new(Mutex::new(None)),
            llm,
        }
    }

    /// Initialize the agent with MCP pool (call this before serving)
    pub async fn init_agent(&self) -> Result<(), anyhow::Error> {
        let pool = McpClientPool::load()?
            .ok_or_else(|| anyhow::anyhow!("No .mcp.json found"))?;

        let mut agent = Agent::new(&self.config.ollama_url, &self.config.model, pool);

        if let Some(ref prompt) = self.config.system_prompt {
            agent = agent.with_system_prompt(prompt);
        }

        let mut guard = self.agent.lock().await;
        *guard = Some(agent);

        Ok(())
    }

    // ========================================================================
    // Chat Tools
    // ========================================================================

    #[tool(description = "Send a message to the LLM and get a response. This is simple chat without tool access.")]
    async fn chat(
        &self,
        Parameters(params): Parameters<ChatParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("chat: {}", params.message);

        // Build messages with optional system prompt
        let response = if let Some(system) = params.system_prompt {
            // For system prompt, we need to use a different approach
            // Simple chat doesn't support system prompts easily, so just prepend
            let full_message = format!("System: {}\n\nUser: {}", system, params.message);
            self.llm
                .chat(&full_message)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        } else {
            self.llm
                .chat(&params.message)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        };

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// Lazily initialize the agent on first use
    async fn ensure_agent(&self) -> Result<(), McpError> {
        let mut guard = self.agent.lock().await;
        if guard.is_none() {
            tracing::info!("Lazily initializing agent with MCP tools...");

            let pool = McpClientPool::load()
                .map_err(|e| McpError::internal_error(format!("Failed to load MCP config: {}", e), None))?
                .ok_or_else(|| McpError::internal_error("No .mcp.json found".to_string(), None))?;

            let mut agent = Agent::new(&self.config.ollama_url, &self.config.model, pool);

            if let Some(ref prompt) = self.config.system_prompt {
                agent = agent.with_system_prompt(prompt);
            }

            *guard = Some(agent);
            tracing::info!("Agent initialized successfully");
        }
        Ok(())
    }

    #[tool(description = "Send a message to the AI agent with access to MCP tools. The agent can use tools like kubernetes, ssh, github to accomplish tasks. Note: First call may take a few seconds to initialize connections.")]
    async fn agent_chat(
        &self,
        Parameters(params): Parameters<AgentChatParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("agent_chat: {}", params.message);

        // Lazily initialize agent on first call
        self.ensure_agent().await?;

        let mut guard = self.agent.lock().await;
        let agent = guard.as_mut().unwrap(); // Safe: ensure_agent guarantees it's Some

        // Update system prompt if provided
        if let Some(ref prompt) = params.system_prompt {
            tracing::info!("Setting system prompt: {}", prompt);
        }

        let response = agent
            .chat(&params.message)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    #[tool(description = "List all available MCP tools that the agent can use. Note: First call may take a few seconds to initialize connections.")]
    async fn list_tools(
        &self,
        Parameters(params): Parameters<ListToolsParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("list_tools: server={:?}", params.server);

        // Lazily initialize agent on first call
        self.ensure_agent().await?;

        let mut guard = self.agent.lock().await;
        let agent = guard.as_mut().unwrap(); // Safe: ensure_agent guarantees it's Some

        let tools = agent
            .tool_names()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // Filter by server if specified (tool names are prefixed with server name)
        let tools: Vec<_> = match params.server {
            Some(ref server) => tools
                .into_iter()
                .filter(|t| t.starts_with(server))
                .collect(),
            None => tools,
        };

        let output = tools.join("\n");
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for AgentMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Agent MCP Server - provides AI assistant capabilities with access to MCP tools. \
                 Use 'chat' for simple LLM interactions, or 'agent_chat' for tool-assisted tasks."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// Start the MCP server on stdio
pub async fn serve(config: ServerConfig) -> anyhow::Result<()> {
    use rmcp::{transport::stdio, ServiceExt};

    tracing::info!("Starting Agent MCP Server");
    tracing::info!("Model: {}", config.model);

    // Create the server - agent initialization is now lazy
    // (happens on first agent_chat call, not at startup)
    let server = AgentMcpServer::new(config);

    // Create stdio transport and serve immediately
    // Don't block on MCP server connections - that would timeout Claude
    let service = server.serve(stdio()).await?;

    tracing::info!("Server running on stdio, waiting for requests...");

    // Wait for shutdown
    service.waiting().await?;

    tracing::info!("Server shutting down");
    Ok(())
}
