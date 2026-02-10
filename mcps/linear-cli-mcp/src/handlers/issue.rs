//! Issue handler implementations

use mcp_common::{text_success, CallToolResult, McpError};

use crate::linear::execute_linear;
#[cfg(feature = "readwrite")]
use crate::params::{IssueCommentAddParams, IssueCreateParams, IssueStartParams};
use crate::params::{IssueListParams, IssueViewParams};

use super::linear_to_mcp_error;

/// List issues with optional filters
pub async fn issue_list(params: IssueListParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["issue", "list"];

    let state_str;
    let sort_str;

    if let Some(ref state) = params.state {
        state_str = state.clone();
        args.extend(["--state", &state_str]);
    }
    if let Some(ref sort) = params.sort {
        sort_str = sort.clone();
        args.extend(["--sort", &sort_str]);
    }

    let team_str;
    if let Some(ref team) = params.team {
        team_str = team.clone();
        args.extend(["--team", &team_str]);
    }

    args.push("--no-pager");

    let output = execute_linear(&args).await.map_err(linear_to_mcp_error)?;
    Ok(text_success(output))
}

/// View a specific issue
pub async fn issue_view(params: IssueViewParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["issue", "view"];

    let issue_id;
    if let Some(ref id) = params.issue_id {
        issue_id = id.clone();
        args.push(&issue_id);
    }

    args.push("--no-pager");

    let output = execute_linear(&args).await.map_err(linear_to_mcp_error)?;
    Ok(text_success(output))
}

/// Create a new issue
#[cfg(feature = "readwrite")]
pub async fn issue_create(params: IssueCreateParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["issue", "create", "--title", &params.title];

    let desc_str;
    if let Some(ref desc) = params.description {
        desc_str = desc.clone();
        args.extend(["--description", &desc_str]);
    }

    let output = execute_linear(&args).await.map_err(linear_to_mcp_error)?;
    let msg = if output.is_empty() {
        "Issue created successfully".to_string()
    } else {
        output
    };
    Ok(text_success(msg))
}

/// Start an issue (changes status and creates a git branch)
#[cfg(feature = "readwrite")]
pub async fn issue_start(params: IssueStartParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["issue", "start"];

    let issue_id;
    if let Some(ref id) = params.issue_id {
        issue_id = id.clone();
        args.push(&issue_id);
    }

    let output = execute_linear(&args).await.map_err(linear_to_mcp_error)?;
    Ok(text_success(output))
}

/// Add a comment to an issue
#[cfg(feature = "readwrite")]
pub async fn issue_comment_add(params: IssueCommentAddParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["issue", "comment", "add"];

    let issue_id;
    if let Some(ref id) = params.issue_id {
        issue_id = id.clone();
        args.push(&issue_id);
    }

    args.extend(["--body", &params.body]);

    let output = execute_linear(&args).await.map_err(linear_to_mcp_error)?;

    let msg = if output.is_empty() {
        "Comment added successfully".to_string()
    } else {
        output
    };
    Ok(text_success(msg))
}

/// Get issue ID from the current git branch
pub async fn issue_id() -> Result<CallToolResult, McpError> {
    let args = vec!["issue", "id"];
    let output = execute_linear(&args).await.map_err(linear_to_mcp_error)?;
    Ok(text_success(output))
}
