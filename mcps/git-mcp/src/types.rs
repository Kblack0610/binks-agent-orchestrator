//! Type definitions for git-mcp

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Response Types
// ============================================================================

/// Information about a commit
#[derive(Debug, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub author_time: DateTime<Utc>,
    pub committer_name: String,
    pub committer_email: String,
    pub committer_time: DateTime<Utc>,
    pub parent_ids: Vec<String>,
}

/// Response for git_status operation
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub repo_path: String,
    pub branch: Option<String>,
    pub head_commit: Option<String>,
    pub is_clean: bool,
    pub staged: Vec<FileStatus>,
    pub modified: Vec<FileStatus>,
    pub untracked: Vec<String>,
    pub conflicted: Vec<String>,
}

/// Status of a file
#[derive(Debug, Serialize, Deserialize)]
pub struct FileStatus {
    pub path: String,
    pub status: String, // "new", "modified", "deleted", "renamed", "typechange"
    pub old_path: Option<String>, // for renamed files
}

/// Response for git_log operation
#[derive(Debug, Serialize, Deserialize)]
pub struct LogResponse {
    pub repo_path: String,
    pub commits: Vec<CommitInfo>,
    pub total_count: usize,
}

/// Response for git_diff operation
#[derive(Debug, Serialize, Deserialize)]
pub struct DiffResponse {
    pub repo_path: String,
    pub from_ref: Option<String>,
    pub to_ref: Option<String>,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub diff: String,
}

/// Response for git_show operation
#[derive(Debug, Serialize, Deserialize)]
pub struct ShowResponse {
    pub commit: CommitInfo,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub diff: Option<String>,
}

/// Branch information
#[derive(Debug, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub commit_id: Option<String>,
    pub upstream: Option<String>,
    pub is_remote: bool,
}

/// Response for git_branch_list operation
#[derive(Debug, Serialize, Deserialize)]
pub struct BranchListResponse {
    pub repo_path: String,
    pub current_branch: Option<String>,
    pub branches: Vec<BranchInfo>,
}

/// Blame line information
#[derive(Debug, Serialize, Deserialize)]
pub struct BlameLine {
    pub line_number: usize,
    pub commit_id: String,
    pub author: String,
    pub author_time: DateTime<Utc>,
    pub content: String,
}

/// Response for git_blame operation
#[derive(Debug, Serialize, Deserialize)]
pub struct BlameResponse {
    pub repo_path: String,
    pub file_path: String,
    pub lines: Vec<BlameLine>,
}

/// Stash entry information
#[derive(Debug, Serialize, Deserialize)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
    pub commit_id: String,
}

/// Response for git_stash operation
#[derive(Debug, Serialize, Deserialize)]
pub struct StashResponse {
    pub repo_path: String,
    pub action: String,
    pub entries: Vec<StashEntry>,
    pub message: Option<String>,
}

/// Remote information
#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: Option<String>,
    pub push_url: Option<String>,
}

/// Response for git_remote_list operation
#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteListResponse {
    pub repo_path: String,
    pub remotes: Vec<RemoteInfo>,
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum GitError {
    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("Reference not found: {0}")]
    RefNotFound(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid reference: {0}")]
    InvalidRef(String),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type GitResult<T> = Result<T, GitError>;
