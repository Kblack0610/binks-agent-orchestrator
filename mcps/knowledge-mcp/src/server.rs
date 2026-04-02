//! MCP Server implementation for knowledge-mcp
//!
//! Exposes FTS5 search, document retrieval, sync, and status tools.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};

use crate::config::KnowledgeConfig;
use crate::docs_store::DocStore;
use crate::handlers;
use crate::params::*;

/// The Knowledge MCP Server
#[derive(Clone)]
pub struct KnowledgeMcpServer {
    store: DocStore,
    config: KnowledgeConfig,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl KnowledgeMcpServer {
    pub fn new(config: KnowledgeConfig) -> Result<Self, anyhow::Error> {
        let db_path = config.db_path();
        let store = DocStore::new(db_path)?;

        Ok(Self {
            store,
            config,
            tool_router: Self::tool_router(),
        })
    }

    #[tool(
        description = "Search indexed documentation using FTS5 full-text search. Returns ranked chunks with provenance (repo, path, commit). Use list_sources first to understand what's indexed, then search_docs for keyword/topic queries. Supports FTS5 syntax: AND, OR, NOT, \"exact phrase\". Filter by repo or kind (instruction, architecture, runbook, lesson, docs)."
    )]
    async fn search_docs(
        &self,
        Parameters(params): Parameters<SearchDocsParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::search_docs(&self.store, params).await
    }

    #[tool(
        description = "Retrieve a full document by ID (from search results) or by repo+path. Returns document metadata and content chunks. Use after search_docs to get the complete document content. Chunks are capped at 20 by default; use chunk_range for pagination."
    )]
    async fn get_doc(
        &self,
        Parameters(params): Parameters<GetDocParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::get_doc(&self.store, params).await
    }

    #[tool(
        description = "Trigger re-ingestion of documentation sources. Reads files from configured repos, chunks markdown by heading, and indexes via FTS5. Hash-based change detection skips unchanged files unless force=true. Filter by repo, source_name, or path_prefix."
    )]
    async fn sync_sources(
        &self,
        Parameters(params): Parameters<SyncSourcesParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::sync_sources(&self.store, &self.config, params).await
    }

    #[tool(
        description = "Show ingestion status: document and chunk counts per source, last sync time, and staleness indicators. Use to verify freshness before trusting search results."
    )]
    async fn get_sync_status(&self) -> Result<CallToolResult, McpError> {
        handlers::get_sync_status(&self.store).await
    }

    #[tool(
        description = "List all configured documentation sources with their repo names, base paths, glob patterns, and current document counts. Call this first to understand what repos and docs are available for searching."
    )]
    async fn list_sources(&self) -> Result<CallToolResult, McpError> {
        handlers::list_sources(&self.store, &self.config).await
    }

    // ── Project Note Tools ──────────────────────────────────────────────

    #[tool(
        description = "List all projects from project notes (~/.notes/dev/projects/). Returns name, status, active version, repo, and overview for each. Filter by status (active/planning/paused)."
    )]
    async fn list_projects(
        &self,
        Parameters(params): Parameters<ListProjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::list_projects(&self.config, params).await
    }

    #[tool(
        description = "Update a project's summary.md in project notes. Can set status, active_version, overview, repo, or append to notes. Changes are re-indexed automatically."
    )]
    async fn update_project_summary(
        &self,
        Parameters(params): Parameters<UpdateProjectSummaryParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::update_project_summary(&self.store, &self.config, params).await
    }

    #[tool(
        description = "Create or update a version checklist file (vX.Y.Z.md) for a project. Can create new versions, add tasks, or check/uncheck existing tasks. Auto-updates summary.md active version on create."
    )]
    async fn update_version(
        &self,
        Parameters(params): Parameters<UpdateVersionParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::update_version(&self.store, &self.config, params).await
    }

    #[tool(
        description = "Add changelog entries for a project version. Writes changelog.md in project notes. Optionally syncs to the project's actual repo by cross-referencing knowledge sources."
    )]
    async fn add_changelog(
        &self,
        Parameters(params): Parameters<AddChangelogParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::add_changelog(&self.store, &self.config, params).await
    }
}

#[tool_handler]
impl rmcp::ServerHandler for KnowledgeMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Cross-repo knowledge index with FTS5 search and project note management. \
                 Indexes documentation from multiple repositories for unified search. \
                 Usage: list_sources -> search_docs -> get_doc -> get_sync_status. \
                 Project notes: list_projects -> update_project_summary / update_version / add_changelog. \
                 Run sync_sources to update the index from source files."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
