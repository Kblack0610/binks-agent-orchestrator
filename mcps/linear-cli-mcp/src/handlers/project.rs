//! Project handler implementations

use mcp_common::{text_success, CallToolResult, McpError};

use crate::linear::execute_linear;

use super::linear_to_mcp_error;

/// List all projects
pub async fn project_list() -> Result<CallToolResult, McpError> {
    let args = vec!["project", "list"];
    let output = execute_linear(&args).await.map_err(linear_to_mcp_error)?;
    Ok(text_success(output))
}
