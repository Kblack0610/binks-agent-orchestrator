//! MCP tool handlers for knowledge-mcp
//!
//! Each handler takes the store + config and params to perform operations.

use mcp_common::{internal_error, json_success, CallToolResult, McpError};

use crate::config::KnowledgeConfig;
use crate::docs_store::DocStore;
use crate::ingest;
use crate::params::*;
use crate::types::*;

pub async fn search_docs(
    store: &DocStore,
    params: SearchDocsParams,
) -> Result<CallToolResult, McpError> {
    let (results, total_matches) = store
        .search(
            &params.query,
            params.repo.as_deref(),
            params.kind.as_deref(),
            params.limit,
        )
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let response = SearchDocsResponse {
        results,
        total_matches,
        query: params.query,
    };

    json_success(&response)
}

pub async fn get_doc(store: &DocStore, params: GetDocParams) -> Result<CallToolResult, McpError> {
    let response = store
        .get_document(
            params.doc_id.as_deref(),
            params.repo.as_deref(),
            params.path.as_deref(),
            params.chunk_range,
        )
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    json_success(&response)
}

pub async fn sync_sources(
    store: &DocStore,
    config: &KnowledgeConfig,
    params: SyncSourcesParams,
) -> Result<CallToolResult, McpError> {
    let response = ingest::run_sync(
        store,
        config,
        params.repo.as_deref(),
        params.source_name.as_deref(),
        params.path_prefix.as_deref(),
        params.force,
    )
    .await
    .map_err(|e| internal_error(e.to_string()))?;

    json_success(&response)
}

pub async fn get_sync_status(store: &DocStore) -> Result<CallToolResult, McpError> {
    let sources = store
        .get_sync_status()
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let response = SyncStatusResponse { sources };

    json_success(&response)
}

pub async fn list_sources(
    store: &DocStore,
    config: &KnowledgeConfig,
) -> Result<CallToolResult, McpError> {
    let mut sources = Vec::new();

    for source_cfg in &config.sources {
        let doc_count = store
            .get_source_doc_count(&source_cfg.name)
            .await
            .unwrap_or(0);

        sources.push(SourceInfo {
            name: source_cfg.name.clone(),
            repo: source_cfg.repo.clone(),
            base_path: source_cfg.base_path.clone(),
            patterns: source_cfg.patterns.clone(),
            exclude_patterns: source_cfg.exclude_patterns.clone(),
            enabled: source_cfg.enabled,
            document_count: doc_count,
        });
    }

    let response = ListSourcesResponse { sources };

    json_success(&response)
}
