//! Pull Request type definitions
//!
//! Structs representing GitHub pull request data as returned by gh CLI.

use super::common::{Label, Milestone, User};
use serde::{Deserialize, Serialize};

/// Represents a GitHub pull request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    /// PR number (unique within repository)
    pub number: u32,

    /// PR title
    pub title: String,

    /// PR state (OPEN, CLOSED, MERGED)
    pub state: String,

    /// PR body/description (markdown)
    #[serde(default)]
    pub body: Option<String>,

    /// PR author
    pub author: User,

    /// Assigned users
    #[serde(default)]
    pub assignees: Vec<User>,

    /// Requested reviewers
    #[serde(default)]
    pub review_requests: Vec<ReviewRequest>,

    /// Applied labels
    #[serde(default)]
    pub labels: Vec<Label>,

    /// Associated milestone
    #[serde(default)]
    pub milestone: Option<Milestone>,

    /// Base branch name
    pub base_ref_name: String,

    /// Head branch name
    pub head_ref_name: String,

    /// Whether PR is in draft mode
    #[serde(default)]
    pub is_draft: bool,

    /// Whether PR is mergeable
    #[serde(default)]
    pub mergeable: Option<String>,

    /// Creation timestamp (ISO 8601)
    pub created_at: String,

    /// Last update timestamp (ISO 8601)
    pub updated_at: String,

    /// Merged timestamp (ISO 8601), if merged
    #[serde(default)]
    pub merged_at: Option<String>,

    /// Closed timestamp (ISO 8601), if closed
    #[serde(default)]
    pub closed_at: Option<String>,

    /// PR URL on GitHub
    pub url: String,

    /// Number of additions
    #[serde(default)]
    pub additions: Option<u32>,

    /// Number of deletions
    #[serde(default)]
    pub deletions: Option<u32>,

    /// Number of changed files
    #[serde(default)]
    pub changed_files: Option<u32>,
}

/// Represents a review request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRequest {
    /// Requested reviewer
    #[serde(flatten)]
    pub user: User,
}

impl PullRequest {
    /// Returns the JSON fields to request from gh CLI for list operations
    pub fn list_fields() -> &'static [&'static str] {
        &[
            "number",
            "title",
            "state",
            "author",
            "assignees",
            "labels",
            "baseRefName",
            "headRefName",
            "isDraft",
            "createdAt",
            "updatedAt",
            "url",
        ]
    }

    /// Returns minimal JSON fields for compact list output
    pub fn list_fields_minimal() -> &'static [&'static str] {
        &[
            "number",
            "title",
            "state",
            "author",
            "headRefName",
            "isDraft",
            "url",
        ]
    }

    /// Returns the JSON fields to request for detailed view operations
    pub fn view_fields() -> &'static [&'static str] {
        &[
            "number",
            "title",
            "state",
            "body",
            "author",
            "assignees",
            "reviewRequests",
            "labels",
            "milestone",
            "baseRefName",
            "headRefName",
            "isDraft",
            "mergeable",
            "createdAt",
            "updatedAt",
            "mergedAt",
            "closedAt",
            "url",
            "additions",
            "deletions",
            "changedFiles",
        ]
    }
}
