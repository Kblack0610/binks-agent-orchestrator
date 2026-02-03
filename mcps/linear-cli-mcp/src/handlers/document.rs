//! Document handler implementations

use mcp_common::{json_success, CallToolResult, McpError};

use crate::linear::execute_linear_json;
use crate::params::{DocumentListParams, DocumentViewParams};

use super::linear_to_mcp_error;

/// List documents with optional filters
pub async fn document_list(params: DocumentListParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["document", "list"];

    let project_str;
    let issue_str;

    if let Some(ref project) = params.project {
        project_str = project.clone();
        args.extend(["--project", &project_str]);
    }
    if let Some(ref issue) = params.issue {
        issue_str = issue.clone();
        args.extend(["--issue", &issue_str]);
    }

    let output: serde_json::Value = execute_linear_json(&args)
        .await
        .map_err(linear_to_mcp_error)?;

    json_success(&output)
}

/// View a document by slug
pub async fn document_view(params: DocumentViewParams) -> Result<CallToolResult, McpError> {
    let args = vec!["document", "view", &params.slug];

    let output: serde_json::Value = execute_linear_json(&args)
        .await
        .map_err(linear_to_mcp_error)?;

    json_success(&output)
}
