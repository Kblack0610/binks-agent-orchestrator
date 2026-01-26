//! Pull request parameter types

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "PR state filter (open, closed, merged, all)")]
    pub state: Option<String>,
    #[schemars(description = "Filter by base branch")]
    pub base: Option<String>,
    #[schemars(description = "Filter by head branch (user:branch)")]
    pub head: Option<String>,
    #[schemars(description = "Filter by label")]
    pub label: Option<String>,
    #[schemars(description = "Maximum number of PRs to return")]
    pub limit: Option<u32>,
    #[schemars(
        description = "Return minimal fields only (number, title, state, author, headRefName, isDraft, url)"
    )]
    pub minimal: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrViewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrCreateParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request title")]
    pub title: String,
    #[schemars(description = "Pull request body in markdown")]
    pub body: Option<String>,
    #[schemars(description = "Base branch to merge into")]
    pub base: Option<String>,
    #[schemars(description = "Head branch with changes")]
    pub head: Option<String>,
    #[schemars(description = "Create as draft PR")]
    pub draft: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrMergeParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "Merge method (merge, squash, rebase)")]
    pub method: Option<String>,
    #[schemars(description = "Delete branch after merge")]
    pub delete_branch: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrDiffParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "Only show names of changed files")]
    pub name_only: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrChecksParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "Only show failed checks")]
    pub failed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrCommentParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "Comment body in markdown")]
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrStatusParams {
    #[schemars(
        description = "Repository in OWNER/REPO format (optional, uses current repo if not specified)"
    )]
    pub repo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrReviewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "Review action: approve, request-changes, comment")]
    pub action: String,
    #[schemars(description = "Review body/comment")]
    pub body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrReadyParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrEditParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "New PR title")]
    pub title: Option<String>,
    #[schemars(description = "New PR body")]
    pub body: Option<String>,
    #[schemars(description = "New base branch")]
    pub base: Option<String>,
    #[schemars(description = "Labels to add (comma-separated)")]
    pub add_labels: Option<String>,
    #[schemars(description = "Labels to remove (comma-separated)")]
    pub remove_labels: Option<String>,
    #[schemars(description = "Assignees to add (comma-separated)")]
    pub add_assignees: Option<String>,
    #[schemars(description = "Reviewers to add (comma-separated)")]
    pub add_reviewers: Option<String>,
}
