//! MCP Server implementation for dual-layer memory
//!
//! This module defines the main MCP server that exposes memory operations as tools.
//! Handler implementations are in the handlers module.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use std::path::PathBuf;

use crate::handlers;
use crate::params::*;
use crate::persistent::PersistentMemory;
use crate::session::SessionMemory;

/// The main Memory MCP Server
#[derive(Clone)]
pub struct MemoryMcpServer {
    session: SessionMemory,
    persistent: PersistentMemory,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Tool Router - Each tool delegates to its handler
// ============================================================================

#[tool_router]
impl MemoryMcpServer {
    pub fn new() -> Result<Self, anyhow::Error> {
        // Default database path: ~/.memory-mcp/memory.db
        let db_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".memory-mcp")
            .join("memory.db");

        let persistent = PersistentMemory::new(db_path)?;

        Ok(Self {
            session: SessionMemory::new(),
            persistent,
            tool_router: Self::tool_router(),
        })
    }

    // ========================================================================
    // Session Layer Tools
    // ========================================================================

    #[tool(
        description = "Record a thinking step in the reasoning chain. Use this to track your thought process and build up context for complex tasks."
    )]
    async fn think(
        &self,
        Parameters(params): Parameters<ThinkParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::think(&self.session, params).await
    }

    #[tool(
        description = "Store a value in working memory for the current session. Useful for tracking intermediate results, context, or state."
    )]
    async fn remember(
        &self,
        Parameters(params): Parameters<RememberParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::remember(&self.session, params).await
    }

    #[tool(description = "Recall a value from working memory by key.")]
    async fn recall(
        &self,
        Parameters(params): Parameters<RecallParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::recall(&self.session, params).await
    }

    #[tool(
        description = "Get the full session context including all thoughts, working memory, and cached tool results."
    )]
    async fn get_context(&self) -> Result<CallToolResult, McpError> {
        handlers::get_context(&self.session).await
    }

    #[tool(
        description = "Cache a tool result with an optional TTL. Useful for avoiding redundant tool calls."
    )]
    async fn cache_tool_result(
        &self,
        Parameters(params): Parameters<CacheToolResultParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::cache_tool_result(&self.session, params).await
    }

    #[tool(
        description = "Clear all session data (thoughts, working memory, tool cache) and start fresh."
    )]
    async fn reset_session(&self) -> Result<CallToolResult, McpError> {
        handlers::reset_session(&self.session).await
    }

    // ========================================================================
    // Persistent Layer Tools
    // ========================================================================

    #[tool(
        description = "Learn about an entity by adding it to the knowledge graph with facts and relations. Information persists across sessions."
    )]
    async fn learn(
        &self,
        Parameters(params): Parameters<LearnParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::learn(&self.persistent, params).await
    }

    #[tool(
        description = "Query the knowledge graph for entities, facts, and relations. Supports pattern matching on entity names (use * as wildcard)."
    )]
    async fn query(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::query(&self.persistent, params).await
    }

    #[tool(
        description = "Summarize and persist the current session. Compresses the thinking chain and saves key insights to persistent storage."
    )]
    async fn summarize_session(&self) -> Result<CallToolResult, McpError> {
        handlers::summarize_session(&self.session, &self.persistent).await
    }

    #[tool(
        description = "Forget an entity from the knowledge graph, removing all its facts and relations."
    )]
    async fn forget(
        &self,
        Parameters(params): Parameters<ForgetParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::forget(&self.persistent, params).await
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for MemoryMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Dual-layer memory MCP server with session and persistent storage. \
                 Session layer provides ephemeral working memory for the current task. \
                 Persistent layer stores a knowledge graph that survives across sessions."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for MemoryMcpServer {
    fn default() -> Self {
        Self::new().expect("Failed to create MemoryMcpServer")
    }
}
