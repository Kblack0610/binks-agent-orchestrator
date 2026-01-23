//! Search and status parameter types

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchPrsParams {
    #[schemars(description = "Repository in OWNER/REPO format (optional, searches all repos if not provided)")]
    pub repo: Option<String>,
    #[schemars(description = "Search query (GitHub search syntax)")]
    pub query: String,
    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchIssuesParams {
    #[schemars(description = "Search query (GitHub search syntax)")]
    pub query: String,
    #[schemars(description = "Repository in OWNER/REPO format (optional)")]
    pub repo: Option<String>,
    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchReposParams {
    #[schemars(description = "Search query (GitHub search syntax)")]
    pub query: String,
    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchCommitsParams {
    #[schemars(description = "Search query (GitHub search syntax)")]
    pub query: String,
    #[schemars(description = "Repository in OWNER/REPO format (optional)")]
    pub repo: Option<String>,
    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StatusParams {
    #[schemars(description = "Filter by organization")]
    pub org: Option<String>,
}
