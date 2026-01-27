//! Issue handler implementations

use mcp_common::{json_success, text_success, CallToolResult, McpError};

use crate::gh::{execute_gh_action, execute_gh_json, execute_gh_raw};
use crate::params::{
    IssueCloseParams, IssueCommentParams, IssueCreateParams, IssueDeleteParams, IssueEditParams,
    IssueListParams, IssueStatusParams, IssueViewParams,
};
use crate::types::Issue;

use super::gh_to_mcp_error;

/// List issues in a GitHub repository
pub async fn issue_list(params: IssueListParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["issue", "list", "-R", &params.repo];

    let state_str;
    let assignee_str;
    let label_str;
    let limit_str;

    if let Some(ref state) = params.state {
        state_str = state.clone();
        args.extend(["-s", &state_str]);
    }
    if let Some(ref assignee) = params.assignee {
        assignee_str = assignee.clone();
        args.extend(["-a", &assignee_str]);
    }
    if let Some(ref label) = params.label {
        label_str = label.clone();
        args.extend(["-l", &label_str]);
    }
    if let Some(limit) = params.limit {
        limit_str = limit.to_string();
        args.extend(["-L", &limit_str]);
    }

    let fields = if params.minimal == Some(true) {
        Issue::list_fields_minimal()
    } else {
        Issue::list_fields()
    };
    let issues: Vec<Issue> = execute_gh_json(&args, fields)
        .await
        .map_err(gh_to_mcp_error)?;

    json_success(&issues)
}

/// View detailed information about a specific GitHub issue
pub async fn issue_view(params: IssueViewParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let args = vec!["issue", "view", &number_str, "-R", &params.repo];

    let issue: Issue = execute_gh_json(&args, Issue::view_fields())
        .await
        .map_err(gh_to_mcp_error)?;

    json_success(&issue)
}

/// Create a new issue in a GitHub repository
pub async fn issue_create(params: IssueCreateParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["issue", "create", "-R", &params.repo, "-t", &params.title];

    let body_str;
    let assignee_str;
    let labels_str;

    if let Some(ref body) = params.body {
        body_str = body.clone();
        args.extend(["-b", &body_str]);
    }
    if let Some(ref assignee) = params.assignee {
        assignee_str = assignee.clone();
        args.extend(["-a", &assignee_str]);
    }
    if let Some(ref labels) = params.labels {
        labels_str = labels.clone();
        args.extend(["-l", &labels_str]);
    }

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    Ok(text_success(output))
}

/// Edit an existing GitHub issue
pub async fn issue_edit(params: IssueEditParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let mut args = vec!["issue", "edit", &number_str, "-R", &params.repo];

    let title_str;
    let body_str;
    let add_labels_str;
    let remove_labels_str;
    let add_assignee_str;

    if let Some(ref title) = params.title {
        title_str = title.clone();
        args.extend(["-t", &title_str]);
    }
    if let Some(ref body) = params.body {
        body_str = body.clone();
        args.extend(["-b", &body_str]);
    }
    if let Some(ref add_labels) = params.add_labels {
        add_labels_str = add_labels.clone();
        args.extend(["--add-label", &add_labels_str]);
    }
    if let Some(ref remove_labels) = params.remove_labels {
        remove_labels_str = remove_labels.clone();
        args.extend(["--remove-label", &remove_labels_str]);
    }
    if let Some(ref add_assignee) = params.add_assignee {
        add_assignee_str = add_assignee.clone();
        args.extend(["--add-assignee", &add_assignee_str]);
    }

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        "Issue updated successfully".to_string()
    } else {
        output
    };
    Ok(text_success(msg))
}

/// Close a GitHub issue
pub async fn issue_close(params: IssueCloseParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let mut args = vec!["issue", "close", &number_str, "-R", &params.repo];

    let reason_str;
    let comment_str;

    if let Some(ref reason) = params.reason {
        reason_str = reason.clone();
        args.extend(["-r", &reason_str]);
    }
    if let Some(ref comment) = params.comment {
        comment_str = comment.clone();
        args.extend(["-c", &comment_str]);
    }

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!("Issue #{} closed successfully", params.number)
    } else {
        output
    };
    Ok(text_success(msg))
}

/// Add a comment to an issue
pub async fn issue_comment(params: IssueCommentParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let args = vec![
        "issue",
        "comment",
        &number_str,
        "-R",
        &params.repo,
        "-b",
        &params.body,
    ];

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!("Comment added to issue #{}", params.number)
    } else {
        output
    };
    Ok(text_success(msg))
}

/// Delete an issue (requires admin permissions)
pub async fn issue_delete(params: IssueDeleteParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let args = vec!["issue", "delete", &number_str, "-R", &params.repo, "--yes"];

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!("Issue #{} deleted successfully", params.number)
    } else {
        output
    };
    Ok(text_success(msg))
}

/// Show status of issues relevant to you
pub async fn issue_status(params: IssueStatusParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["issue", "status"];

    let repo_str;
    if let Some(ref repo) = params.repo {
        repo_str = repo.clone();
        args.extend(["-R", &repo_str]);
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(text_success(output))
}
