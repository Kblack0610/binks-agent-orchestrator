//! MCP Server implementation for agent registry
//!
//! This module defines the main MCP server that exposes agent registry operations as tools.
//! Handler implementations are in the handlers module.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use std::path::PathBuf;

use crate::handlers;
use crate::params::*;
use crate::repository::AgentRegistryRepository;

/// The main Agent Registry MCP Server
#[derive(Clone)]
pub struct AgentRegistryMcpServer {
    repository: AgentRegistryRepository,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Tool Router - Each tool delegates to its handler
// ============================================================================

#[tool_router]
impl AgentRegistryMcpServer {
    pub fn new() -> Result<Self, anyhow::Error> {
        // Database path: ~/.binks/conversations.db (shared with agent and task-mcp)
        let db_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".binks")
            .join("conversations.db");

        let repository = AgentRegistryRepository::new(db_path)?;

        Ok(Self {
            repository,
            tool_router: Self::tool_router(),
        })
    }

    // ========================================================================
    // Agent Lifecycle
    // ========================================================================

    #[tool(description = "Register a new agent, returns unique agent_id (UUID)")]
    async fn register_agent(
        &self,
        Parameters(params): Parameters<RegisterAgentParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::register_agent(&self.repository, params).await
    }

    #[tool(description = "Graceful deregister: marks agent deregistered, releases all claims")]
    async fn deregister_agent(
        &self,
        Parameters(params): Parameters<DeregisterAgentParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::deregister_agent(&self.repository, params).await
    }

    #[tool(description = "Heartbeat to prove liveness, optionally update status/project")]
    async fn heartbeat(
        &self,
        Parameters(params): Parameters<HeartbeatParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::heartbeat(&self.repository, params).await
    }

    #[tool(description = "Update agent fields (status, project, capabilities) mid-session")]
    async fn update_agent(
        &self,
        Parameters(params): Parameters<UpdateAgentParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::update_agent(&self.repository, params).await
    }

    // ========================================================================
    // Port Management
    // ========================================================================

    #[tool(description = "Claim a port (first-come-first-served, returns conflict if taken)")]
    async fn claim_port(
        &self,
        Parameters(params): Parameters<ClaimPortParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::claim_port(&self.repository, params).await
    }

    #[tool(description = "Release a claimed port")]
    async fn release_port(
        &self,
        Parameters(params): Parameters<ReleasePortParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::release_port(&self.repository, params).await
    }

    #[tool(description = "Query who currently holds a specific port")]
    async fn who_has_port(
        &self,
        Parameters(params): Parameters<WhoHasPortParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::who_has_port(&self.repository, params).await
    }

    // ========================================================================
    // Resource Management
    // ========================================================================

    #[tool(description = "Claim a resource (directory, project, branch) with exclusive/shared lock")]
    async fn claim_resource(
        &self,
        Parameters(params): Parameters<ClaimResourceParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::claim_resource(&self.repository, params).await
    }

    #[tool(description = "Release a specific resource claim by claim ID")]
    async fn release_resource(
        &self,
        Parameters(params): Parameters<ReleaseResourceParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::release_resource(&self.repository, params).await
    }

    #[tool(description = "Release all resource and port claims held by an agent")]
    async fn release_all_resources(
        &self,
        Parameters(params): Parameters<ReleaseAllResourcesParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::release_all_resources(&self.repository, params).await
    }

    // ========================================================================
    // Queries
    // ========================================================================

    #[tool(description = "List registered agents with filters (status, type, project)")]
    async fn list_agents(
        &self,
        Parameters(params): Parameters<ListAgentsParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::list_agents(&self.repository, params).await
    }

    #[tool(description = "Fetch a single agent record by agent_id")]
    async fn get_agent(
        &self,
        Parameters(params): Parameters<GetAgentParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::get_agent(&self.repository, params).await
    }

    #[tool(description = "Query which agents are working on a given repo/directory/project")]
    async fn who_is_working_on(
        &self,
        Parameters(params): Parameters<WhoIsWorkingOnParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::who_is_working_on(&self.repository, params).await
    }

    #[tool(description = "List resource and port claims with optional filters")]
    async fn list_claims(
        &self,
        Parameters(params): Parameters<ListClaimsParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::list_claims(&self.repository, params).await
    }

    #[tool(description = "Expire stale agents and release their claims, supports dry_run")]
    async fn cleanup_stale(
        &self,
        Parameters(params): Parameters<CleanupStaleParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::cleanup_stale(&self.repository, params).await
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for AgentRegistryMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Agent registry MCP server for service discovery and resource coordination. \
                 Agents register on startup, heartbeat for liveness, claim ports and resources \
                 to avoid conflicts, and query who else is running. \
                 Shares ~/.binks/conversations.db with task-mcp and the agent."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for AgentRegistryMcpServer {
    fn default() -> Self {
        Self::new().expect("Failed to create AgentRegistryMcpServer")
    }
}
