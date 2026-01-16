//! Issue type definitions
//!
//! Structs representing GitHub issue data as returned by gh CLI.
//! These types mirror the JSON structure from `gh issue list --json`.

use serde::{Deserialize, Serialize};
use super::common::{Label, Milestone, User};

/// Represents a GitHub issue
///
/// This struct contains the fields commonly returned by gh CLI
/// for issue operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// Issue number (unique within repository)
    pub number: u32,

    /// Issue title
    pub title: String,

    /// Issue state (OPEN, CLOSED)
    pub state: String,

    /// Issue body/description (markdown)
    #[serde(default)]
    pub body: Option<String>,

    /// Issue author
    pub author: User,

    /// Assigned users
    #[serde(default)]
    pub assignees: Vec<User>,

    /// Applied labels
    #[serde(default)]
    pub labels: Vec<Label>,

    /// Associated milestone
    #[serde(default)]
    pub milestone: Option<Milestone>,

    /// Creation timestamp (ISO 8601)
    pub created_at: String,

    /// Last update timestamp (ISO 8601)
    pub updated_at: String,

    /// Closed timestamp (ISO 8601), if closed
    #[serde(default)]
    pub closed_at: Option<String>,

    /// Issue URL on GitHub
    pub url: String,

    /// Number of comments
    #[serde(default)]
    pub comments: Option<u32>,
}

impl Issue {
    /// Returns the JSON fields to request from gh CLI for list operations
    ///
    /// These fields provide a good balance of information without
    /// making expensive API calls.
    pub fn list_fields() -> &'static [&'static str] {
        &[
            "number",
            "title",
            "state",
            "author",
            "assignees",
            "labels",
            "createdAt",
            "updatedAt",
            "url",
            "comments",
        ]
    }

    /// Returns the JSON fields to request for detailed view operations
    ///
    /// Includes body and milestone for complete issue details.
    pub fn view_fields() -> &'static [&'static str] {
        &[
            "number",
            "title",
            "state",
            "body",
            "author",
            "assignees",
            "labels",
            "milestone",
            "createdAt",
            "updatedAt",
            "closedAt",
            "url",
            "comments",
        ]
    }
}
