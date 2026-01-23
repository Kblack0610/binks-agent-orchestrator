//! Repository parameter types

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RepoListParams {
    #[schemars(description = "Filter by owner/organization")]
    pub owner: Option<String>,
    #[schemars(description = "Visibility filter (public, private, internal)")]
    pub visibility: Option<String>,
    #[schemars(description = "Filter by primary language")]
    pub language: Option<String>,
    #[schemars(description = "Maximum number of repos to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RepoViewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
}
