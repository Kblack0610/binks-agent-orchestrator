//! Team handler implementations

use mcp_common::{text_success, CallToolResult, McpError};

use crate::linear::execute_linear;

use super::linear_to_mcp_error;

/// List all teams
pub async fn team_list() -> Result<CallToolResult, McpError> {
    let args = vec!["team", "list"];
    let output = execute_linear(&args).await.map_err(linear_to_mcp_error)?;
    Ok(text_success(output))
}

/// List members of the current team
pub async fn team_members() -> Result<CallToolResult, McpError> {
    let args = vec!["team", "members"];
    let output = execute_linear(&args).await.map_err(linear_to_mcp_error)?;
    Ok(text_success(output))
}
