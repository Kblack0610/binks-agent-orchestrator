//! Release, label, cache, secret, and variable parameter types

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Maximum number of releases to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseViewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Release tag name (e.g., v1.0.0). Use 'latest' for most recent.")]
    pub tag: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseCreateParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Tag name for the release (e.g., v1.0.0)")]
    pub tag: String,
    #[schemars(description = "Release title")]
    pub title: Option<String>,
    #[schemars(description = "Release notes body")]
    pub notes: Option<String>,
    #[schemars(description = "Target branch or commit SHA")]
    pub target: Option<String>,
    #[schemars(description = "Create as draft release")]
    pub draft: Option<bool>,
    #[schemars(description = "Mark as prerelease")]
    pub prerelease: Option<bool>,
    #[schemars(description = "Auto-generate release notes")]
    pub generate_notes: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LabelListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Maximum number of labels to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LabelCreateParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Label name")]
    pub name: String,
    #[schemars(description = "Label color (hex without #, e.g., 'ff0000')")]
    pub color: Option<String>,
    #[schemars(description = "Label description")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CacheListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Maximum number of caches to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SecretListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct VariableListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
}
