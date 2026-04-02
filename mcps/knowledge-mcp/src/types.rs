//! Domain types and response types for knowledge-mcp

use serde::{Deserialize, Serialize};

// ============================================================================
// Domain Types
// ============================================================================

/// A configured ingestion source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub repo: String,
    pub base_path: String,
    pub source_type: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// A single ingested document (one file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub source_id: String,
    pub repo: String,
    pub file_path: String,
    pub relative_path: String,
    pub source_type: String,
    pub kind: String,
    pub priority: i32,
    pub title: Option<String>,
    pub content: String,
    pub content_hash: String,
    pub file_mtime: Option<String>,
    pub commit_hash: Option<String>,
    pub sync_time: String,
    pub chunk_count: i32,
}

/// A chunk of a document (section-level split)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub heading: Option<String>,
    pub content: String,
    pub byte_offset: i64,
    pub byte_length: i64,
}

// ============================================================================
// Response Types
// ============================================================================

/// Result from search_docs
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchDocsResponse {
    pub results: Vec<SearchResult>,
    pub total_matches: usize,
    pub query: String,
}

/// A single search result with provenance
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub doc_id: String,
    pub chunk_id: String,
    pub repo: String,
    pub relative_path: String,
    pub heading: Option<String>,
    pub snippet: String,
    pub kind: String,
    pub priority: i32,
    pub rank: f64,
    pub sync_time: String,
    pub commit_hash: Option<String>,
    pub source_name: String,
    pub stale: bool,
}

/// Result from get_doc
#[derive(Debug, Serialize, Deserialize)]
pub struct GetDocResponse {
    pub document: DocumentInfo,
    pub chunks: Vec<ChunkInfo>,
    pub truncated: bool,
}

/// Document metadata for get_doc response
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: String,
    pub repo: String,
    pub relative_path: String,
    pub kind: String,
    pub priority: i32,
    pub title: Option<String>,
    pub content_hash: String,
    pub sync_time: String,
    pub commit_hash: Option<String>,
    pub chunk_count: i32,
    pub source_name: String,
    pub stale: bool,
}

/// Chunk content for get_doc response
#[derive(Debug, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub id: String,
    pub chunk_index: i32,
    pub heading: Option<String>,
    pub content: String,
}

/// Result from sync_sources
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResponse {
    pub sources_synced: usize,
    pub documents_added: usize,
    pub documents_updated: usize,
    pub documents_unchanged: usize,
    pub documents_skipped: usize,
    pub documents_removed: usize,
    pub duration_ms: u64,
}

/// Result from get_sync_status
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncStatusResponse {
    pub sources: Vec<SourceStatus>,
}

/// Per-source sync status
#[derive(Debug, Serialize, Deserialize)]
pub struct SourceStatus {
    pub name: String,
    pub repo: String,
    pub enabled: bool,
    pub document_count: usize,
    pub chunk_count: usize,
    pub last_sync_time: Option<String>,
    pub stale_count: usize,
    pub oldest_sync: Option<String>,
    pub newest_sync: Option<String>,
}

/// Result from list_sources
#[derive(Debug, Serialize, Deserialize)]
pub struct ListSourcesResponse {
    pub sources: Vec<SourceInfo>,
}

/// Source info for list_sources response
#[derive(Debug, Serialize, Deserialize)]
pub struct SourceInfo {
    pub name: String,
    pub repo: String,
    pub base_path: String,
    pub patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub enabled: bool,
    pub document_count: usize,
}

// ============================================================================
// Project Note Types
// ============================================================================

/// Parsed project summary from summary.md
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub overview: String,
    pub status: String,
    pub active_version: Option<String>,
    pub repo: Option<String>,
    pub notes: String,
}

/// Response from list_projects
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectListEntry>,
}

/// A single project in the list
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectListEntry {
    pub name: String,
    pub status: String,
    pub active_version: Option<String>,
    pub repo: Option<String>,
    pub overview: String,
}

/// Response from update_project_summary
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectUpdateResponse {
    pub project: String,
    pub updated_fields: Vec<String>,
    pub file_path: String,
}

/// Response from update_version
#[derive(Debug, Serialize, Deserialize)]
pub struct VersionUpdateResponse {
    pub project: String,
    pub version: String,
    pub created: bool,
    pub tasks_added: usize,
    pub tasks_toggled: usize,
    pub file_path: String,
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum KnowledgeError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Other(String),
}
