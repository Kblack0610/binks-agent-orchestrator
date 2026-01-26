//! Issue-related parameter types

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue state filter (open, closed, all)")]
    pub state: Option<String>,
    #[schemars(description = "Filter by assignee username")]
    pub assignee: Option<String>,
    #[schemars(description = "Filter by label name")]
    pub label: Option<String>,
    #[schemars(description = "Maximum number of issues to return (default: 30)")]
    pub limit: Option<u32>,
    #[schemars(description = "Return minimal fields only (number, title, state, author, url)")]
    pub minimal: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueViewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue number")]
    pub number: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueCreateParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue title")]
    pub title: String,
    #[schemars(description = "Issue body in markdown")]
    pub body: Option<String>,
    #[schemars(description = "Assignee username (@me for self)")]
    pub assignee: Option<String>,
    #[schemars(description = "Labels to add (comma-separated)")]
    pub labels: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueEditParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue number")]
    pub number: u32,
    #[schemars(description = "New issue title")]
    pub title: Option<String>,
    #[schemars(description = "New issue body")]
    pub body: Option<String>,
    #[schemars(description = "Labels to add (comma-separated)")]
    pub add_labels: Option<String>,
    #[schemars(description = "Labels to remove (comma-separated)")]
    pub remove_labels: Option<String>,
    #[schemars(description = "Assignee to add")]
    pub add_assignee: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueCloseParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue number")]
    pub number: u32,
    #[schemars(description = "Close reason (completed, not_planned)")]
    pub reason: Option<String>,
    #[schemars(description = "Comment to add when closing")]
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueCommentParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue number")]
    pub number: u32,
    #[schemars(description = "Comment body in markdown")]
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueDeleteParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue number")]
    pub number: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueStatusParams {
    #[schemars(
        description = "Repository in OWNER/REPO format (optional, uses current repo if not specified)"
    )]
    pub repo: Option<String>,
}
