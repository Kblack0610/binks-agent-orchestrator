//! Document-related parameter types

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for listing documents
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DocumentListParams {
    #[schemars(description = "Filter documents by project name")]
    pub project: Option<String>,

    #[schemars(description = "Filter documents by issue identifier")]
    pub issue: Option<String>,
}

/// Parameters for viewing a document
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DocumentViewParams {
    #[schemars(description = "Document slug identifier")]
    pub slug: String,
}
