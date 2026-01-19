//! MCP Server implementation for dual-layer memory

use chrono::Utc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

use crate::persistent::PersistentMemory;
use crate::session::SessionMemory;
use crate::types::{
    ForgetResponse, LearnResponse, MemoryValue, QueryResponse,
    RecallResponse, RememberResponse, SummarizeResponse, ThinkResponse,
};

/// The main Memory MCP Server
#[derive(Clone)]
pub struct MemoryMcpServer {
    session: SessionMemory,
    persistent: PersistentMemory,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Parameter Types - Session Layer
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThinkParams {
    #[schemars(description = "The thinking content/observation to record")]
    pub thought: String,

    #[schemars(description = "Confidence level from 0.0 to 1.0 (default: 0.5)")]
    #[serde(default = "default_confidence")]
    pub confidence: f32,

    #[schemars(description = "Optional ID of a previous thought this revises")]
    pub revises: Option<String>,
}

fn default_confidence() -> f32 {
    0.5
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RememberParams {
    #[schemars(description = "The key to store the value under")]
    pub key: String,

    #[schemars(description = "The value to store (string, number, boolean, array, or object)")]
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RecallParams {
    #[schemars(description = "The key to retrieve")]
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CacheToolResultParams {
    #[schemars(description = "Name of the tool that was called")]
    pub tool_name: String,

    #[schemars(description = "Summary of the tool result")]
    pub summary: String,

    #[schemars(description = "Optional TTL in seconds (how long to keep cached)")]
    pub ttl_seconds: Option<u64>,
}

// ============================================================================
// Parameter Types - Persistent Layer
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LearnParams {
    #[schemars(description = "Entity name (e.g., 'project:myapp', 'user:john')")]
    pub entity: String,

    #[schemars(description = "Entity type (e.g., 'project', 'user', 'concept')")]
    pub entity_type: String,

    #[schemars(description = "Facts about the entity as key-value pairs")]
    #[serde(default)]
    pub facts: Vec<FactInput>,

    #[schemars(description = "Relations to other entities")]
    #[serde(default)]
    pub relations: Vec<RelationInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FactInput {
    #[schemars(description = "Fact key/attribute name")]
    pub key: String,

    #[schemars(description = "Fact value")]
    pub value: String,

    #[schemars(description = "Source of fact: 'user', 'inferred', 'learned'")]
    #[serde(default = "default_source")]
    pub source: String,

    #[schemars(description = "Confidence score from 0.0 to 1.0")]
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

fn default_source() -> String {
    "learned".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelationInput {
    #[schemars(description = "Target entity name")]
    pub to_entity: String,

    #[schemars(description = "Relation type (e.g., 'depends_on', 'created_by')")]
    pub relation_type: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct QueryParams {
    #[schemars(description = "Entity name pattern to search (optional, uses LIKE matching)")]
    pub entity_pattern: Option<String>,

    #[schemars(description = "Filter by entity type")]
    pub entity_type: Option<String>,

    #[schemars(description = "Include relations in results")]
    #[serde(default = "default_true")]
    pub include_relations: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ForgetParams {
    #[schemars(description = "Entity name to forget/delete")]
    pub entity: String,
}

// ============================================================================
// Tool Router Implementation
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
        let thought = self
            .session
            .think(params.thought, params.confidence, params.revises)
            .await;

        let step_number = self.session.thought_count().await;

        let response = ThinkResponse {
            thought_id: thought.id,
            step_number,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Store a value in working memory for the current session. Useful for tracking intermediate results, context, or state."
    )]
    async fn remember(
        &self,
        Parameters(params): Parameters<RememberParams>,
    ) -> Result<CallToolResult, McpError> {
        let memory_value = MemoryValue::from(params.value);
        self.session.remember(params.key.clone(), memory_value).await;

        let response = RememberResponse {
            key: params.key,
            success: true,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Recall a value from working memory by key."
    )]
    async fn recall(
        &self,
        Parameters(params): Parameters<RecallParams>,
    ) -> Result<CallToolResult, McpError> {
        let value = self.session.recall(&params.key).await;

        let response = RecallResponse {
            key: params.key,
            found: value.is_some(),
            value,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Get the full session context including all thoughts, working memory, and cached tool results."
    )]
    async fn get_context(&self) -> Result<CallToolResult, McpError> {
        let context = self.session.get_full_context().await;

        let json = serde_json::to_string_pretty(&context)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Cache a tool result with an optional TTL. Useful for avoiding redundant tool calls."
    )]
    async fn cache_tool_result(
        &self,
        Parameters(params): Parameters<CacheToolResultParams>,
    ) -> Result<CallToolResult, McpError> {
        self.session
            .cache_tool_result(params.tool_name.clone(), params.summary, params.ttl_seconds)
            .await;

        let response = serde_json::json!({
            "cached": true,
            "tool_name": params.tool_name,
            "ttl_seconds": params.ttl_seconds
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Clear all session data (thoughts, working memory, tool cache) and start fresh."
    )]
    async fn reset_session(&self) -> Result<CallToolResult, McpError> {
        self.session.reset().await;

        let response = serde_json::json!({
            "reset": true,
            "new_session_id": self.session.session_id().await
        });

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
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
        // Get or create entity
        let entity = self
            .persistent
            .get_or_create_entity(&params.entity, &params.entity_type)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut facts_added = 0;
        let mut relations_added = 0;

        // Add facts
        for fact in params.facts {
            self.persistent
                .add_fact(&entity.id, &fact.key, &fact.value, &fact.source, fact.confidence)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            facts_added += 1;
        }

        // Add relations
        for relation in params.relations {
            // Get or create target entity (with unknown type if not exists)
            let target = self
                .persistent
                .get_or_create_entity(&relation.to_entity, "unknown")
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            self.persistent
                .add_relation(&entity.id, &target.id, &relation.relation_type)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            relations_added += 1;
        }

        let response = LearnResponse {
            entity_id: entity.id,
            facts_added,
            relations_added,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Query the knowledge graph for entities, facts, and relations. Supports pattern matching on entity names (use * as wildcard)."
    )]
    async fn query(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<CallToolResult, McpError> {
        // Build search pattern - default to all if no pattern specified
        let pattern = params.entity_pattern.unwrap_or_else(|| "%".to_string());

        // Query entities (returns EntityWithFacts directly)
        let entities_with_facts = self
            .persistent
            .query_entities(&pattern)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // Collect relations if requested
        let mut all_relations = Vec::new();
        if params.include_relations {
            for ewf in &entities_with_facts {
                let relations = self
                    .persistent
                    .get_relations(&ewf.entity.id)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                all_relations.extend(relations);
            }
        }

        let response = QueryResponse {
            entities: entities_with_facts,
            relations: all_relations,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Summarize and persist the current session. Compresses the thinking chain and saves key insights to persistent storage."
    )]
    async fn summarize_session(&self) -> Result<CallToolResult, McpError> {
        let session_id = self.session.session_id().await;
        let started_at = self.session.started_at().await;
        let thought_count = self.session.thought_count().await;
        let session_end = Utc::now();

        // Get session summary
        let content = self.session.get_session_summary().await;

        // Save to persistent storage
        let summary = self
            .persistent
            .save_summary(&session_id, &content, thought_count, started_at, session_end)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let response = SummarizeResponse {
            summary_id: summary.id,
            content: summary.content,
            thoughts_compressed: thought_count,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Forget an entity from the knowledge graph, removing all its facts and relations."
    )]
    async fn forget(
        &self,
        Parameters(params): Parameters<ForgetParams>,
    ) -> Result<CallToolResult, McpError> {
        let (facts_removed, relations_removed) = self
            .persistent
            .delete_entity(&params.entity)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let response = ForgetResponse {
            entity_name: params.entity,
            deleted: facts_removed > 0 || relations_removed > 0,
            facts_removed,
            relations_removed,
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
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
