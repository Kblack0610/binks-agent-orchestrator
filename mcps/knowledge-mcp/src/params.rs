//! Parameter types for knowledge-mcp MCP tools

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn default_limit() -> u32 {
    10
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchDocsParams {
    #[schemars(
        description = "FTS5 search query. Supports AND, OR, NOT, and \"phrase\" syntax. Example: 'deployment AND kubernetes'"
    )]
    pub query: String,

    #[schemars(
        description = "Filter results to a specific repo name (e.g. 'bnb-platform', 'binks-agent')"
    )]
    pub repo: Option<String>,

    #[schemars(
        description = "Filter by document kind: instruction, architecture, runbook, lesson, plan, docs"
    )]
    pub kind: Option<String>,

    #[schemars(description = "Maximum results to return (default 10, max 50)")]
    #[serde(default = "default_limit")]
    pub limit: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetDocParams {
    #[schemars(description = "Document ID (from search results). Use this OR repo+path.")]
    pub doc_id: Option<String>,

    #[schemars(description = "Repository name. Use with 'path' for repo+path lookup.")]
    pub repo: Option<String>,

    #[schemars(
        description = "Relative path within the repo (e.g. 'docs/ARCHITECTURE.md'). Use with 'repo'."
    )]
    pub path: Option<String>,

    #[schemars(description = "Optional [start, end] chunk range to limit response size")]
    pub chunk_range: Option<[u32; 2]>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SyncSourcesParams {
    #[schemars(description = "Filter sync to a specific repo")]
    pub repo: Option<String>,

    #[schemars(description = "Filter sync to a specific source name from config")]
    pub source_name: Option<String>,

    #[schemars(description = "Filter sync to paths starting with this prefix")]
    pub path_prefix: Option<String>,

    #[schemars(description = "Force re-ingest even if content hash is unchanged")]
    #[serde(default)]
    pub force: bool,
}
