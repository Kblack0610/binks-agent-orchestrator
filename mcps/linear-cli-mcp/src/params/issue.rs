//! Issue-related parameter types

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for listing issues
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueListParams {
    #[schemars(description = "Filter by issue state (e.g., 'started', 'unstarted', 'completed', 'canceled')")]
    pub state: Option<String>,

    #[schemars(description = "Sort order (e.g., 'created', 'updated', 'priority')")]
    pub sort: Option<String>,

    #[schemars(description = "Filter by team key (e.g., 'ENG')")]
    pub team: Option<String>,
}

/// Parameters for viewing an issue
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueViewParams {
    #[schemars(description = "Issue identifier (e.g., 'ENG-123'). If omitted, uses the current git branch")]
    pub issue_id: Option<String>,
}

/// Parameters for creating an issue
#[cfg(feature = "readwrite")]
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueCreateParams {
    #[schemars(description = "Title for the new issue")]
    pub title: String,

    #[schemars(description = "Description for the new issue")]
    pub description: Option<String>,
}

/// Parameters for starting an issue
#[cfg(feature = "readwrite")]
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueStartParams {
    #[schemars(description = "Issue identifier to start (e.g., 'ENG-123'). If omitted, uses the current git branch")]
    pub issue_id: Option<String>,
}

/// Parameters for adding a comment to an issue
#[cfg(feature = "readwrite")]
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueCommentAddParams {
    #[schemars(description = "Issue identifier (e.g., 'ENG-123'). If omitted, uses the current git branch")]
    pub issue_id: Option<String>,

    #[schemars(description = "Comment body text (markdown supported)")]
    pub body: String,
}
