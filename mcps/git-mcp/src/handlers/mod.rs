//! Git operation handlers
//!
//! This module contains the implementation logic for git operations,
//! separate from the MCP tool definitions.

mod core;
mod extras;

pub use core::*;
pub use extras::*;

use chrono::{DateTime, TimeZone, Utc};
use git2::Repository;
use rmcp::ErrorData as McpError;

use crate::types::{CommitInfo, GitError, GitResult};

/// Convert GitError to MCP error with appropriate error codes
pub fn git_error_to_mcp(e: GitError) -> McpError {
    match &e {
        GitError::RepoNotFound(_) | GitError::RefNotFound(_) | GitError::FileNotFound(_) => {
            McpError::invalid_params(e.to_string(), None)
        }
        GitError::InvalidRef(_) | GitError::InvalidPath(_) => {
            McpError::invalid_request(e.to_string(), None)
        }
        _ => McpError::internal_error(e.to_string(), None),
    }
}

/// Open a git repository, discovering from the given path
pub fn open_repo(path: Option<&str>) -> GitResult<Repository> {
    let repo_path = path.unwrap_or(".");
    Repository::discover(repo_path).map_err(|_| GitError::RepoNotFound(repo_path.to_string()))
}

/// Convert git2::Time to chrono DateTime
pub fn git_time_to_datetime(time: git2::Time) -> DateTime<Utc> {
    Utc.timestamp_opt(time.seconds(), 0)
        .single()
        .unwrap_or_else(Utc::now)
}

/// Convert a git2::Commit to our CommitInfo type
pub fn commit_to_info(commit: &git2::Commit) -> CommitInfo {
    CommitInfo {
        id: commit.id().to_string(),
        short_id: commit.id().to_string()[..7].to_string(),
        message: commit.message().unwrap_or("").to_string(),
        author_name: commit.author().name().unwrap_or("").to_string(),
        author_email: commit.author().email().unwrap_or("").to_string(),
        author_time: git_time_to_datetime(commit.author().when()),
        committer_name: commit.committer().name().unwrap_or("").to_string(),
        committer_email: commit.committer().email().unwrap_or("").to_string(),
        committer_time: git_time_to_datetime(commit.committer().when()),
        parent_ids: commit.parent_ids().map(|id| id.to_string()).collect(),
    }
}

/// Convert git2::Delta to status string
#[allow(dead_code)]
pub fn delta_to_status(delta: git2::Delta) -> &'static str {
    match delta {
        git2::Delta::Added => "new",
        git2::Delta::Deleted => "deleted",
        git2::Delta::Modified => "modified",
        git2::Delta::Renamed => "renamed",
        git2::Delta::Copied => "copied",
        git2::Delta::Typechange => "typechange",
        _ => "unknown",
    }
}
