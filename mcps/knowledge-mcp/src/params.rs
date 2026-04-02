//! Parameter types for knowledge-mcp MCP tools

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::ChangelogEntry;

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

// ============================================================================
// Project Note Params
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListProjectsParams {
    #[schemars(description = "Filter by status: active, planning, paused. Omit for all.")]
    pub status: Option<String>,

    #[schemars(description = "Include archived projects (default false)")]
    #[serde(default)]
    pub include_archived: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateProjectSummaryParams {
    #[schemars(description = "Project folder name (e.g. 'dodginballs')")]
    pub project: String,

    #[schemars(description = "New status: active, planning, or paused")]
    pub status: Option<String>,

    #[schemars(description = "New active version string (e.g. 'v1.1.0')")]
    pub active_version: Option<String>,

    #[schemars(description = "New overview text")]
    pub overview: Option<String>,

    #[schemars(description = "Text to append to the Notes section")]
    pub notes: Option<String>,

    #[schemars(description = "Set the repo field (e.g. 'kblack0610/dodginballs')")]
    pub repo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateVersionParams {
    #[schemars(description = "Project folder name")]
    pub project: String,

    #[schemars(
        description = "Version string (e.g. '1.1.0' — the 'v' prefix and .md are added automatically)"
    )]
    pub version: String,

    #[schemars(description = "Create a new version file (fails if it already exists)")]
    #[serde(default)]
    pub create: bool,

    #[schemars(description = "Description/heading for a new version file")]
    pub description: Option<String>,

    #[schemars(description = "New unchecked tasks to add")]
    pub add_tasks: Option<Vec<String>>,

    #[schemars(description = "Task text substrings to mark as checked")]
    pub check_tasks: Option<Vec<String>>,

    #[schemars(description = "Task text substrings to mark as unchecked")]
    pub uncheck_tasks: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AddChangelogParams {
    #[schemars(description = "Project folder name")]
    pub project: String,

    #[schemars(description = "Version string (e.g. '1.1.0')")]
    pub version: String,

    #[schemars(description = "Changelog entries with category and description")]
    pub entries: Vec<ChangelogEntryParam>,

    #[schemars(
        description = "Also write changelog.md to the project's actual repo (resolved via knowledge sources)"
    )]
    #[serde(default)]
    pub sync_to_repo: bool,

    #[schemars(
        description = "Explicit local repo path override (used if repo isn't a knowledge source)"
    )]
    pub repo_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChangelogEntryParam {
    #[schemars(
        description = "Category: Added, Changed, Fixed, Removed, Deprecated, Security"
    )]
    pub category: String,

    #[schemars(description = "Description of the change")]
    pub description: String,
}

impl From<ChangelogEntryParam> for ChangelogEntry {
    fn from(p: ChangelogEntryParam) -> Self {
        Self {
            category: p.category,
            description: p.description,
        }
    }
}
