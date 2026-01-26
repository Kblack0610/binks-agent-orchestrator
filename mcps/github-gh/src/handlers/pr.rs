//! Pull Request handler implementations

use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;

use crate::gh::{
    execute_gh_action, execute_gh_json, execute_gh_raw, execute_gh_raw_with_exit_code,
};
use crate::params::{
    PrChecksParams, PrCommentParams, PrCreateParams, PrDiffParams, PrEditParams, PrListParams,
    PrMergeParams, PrReadyParams, PrReviewParams, PrStatusParams, PrViewParams,
};
use crate::types::PullRequest;

use super::gh_to_mcp_error;

/// List pull requests in a GitHub repository
pub async fn pr_list(params: PrListParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["pr", "list", "-R", &params.repo];

    let state_str;
    let base_str;
    let head_str;
    let label_str;
    let limit_str;

    if let Some(ref state) = params.state {
        state_str = state.clone();
        args.extend(["-s", &state_str]);
    }
    if let Some(ref base) = params.base {
        base_str = base.clone();
        args.extend(["-B", &base_str]);
    }
    if let Some(ref head) = params.head {
        head_str = head.clone();
        args.extend(["-H", &head_str]);
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
        PullRequest::list_fields_minimal()
    } else {
        PullRequest::list_fields()
    };
    let prs: Vec<PullRequest> = execute_gh_json(&args, fields)
        .await
        .map_err(gh_to_mcp_error)?;

    let json = serde_json::to_string(&prs)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// View detailed information about a pull request
pub async fn pr_view(params: PrViewParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let args = vec!["pr", "view", &number_str, "-R", &params.repo];

    let pr: PullRequest = execute_gh_json(&args, PullRequest::view_fields())
        .await
        .map_err(gh_to_mcp_error)?;

    let json = serde_json::to_string(&pr)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Create a new pull request
pub async fn pr_create(params: PrCreateParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["pr", "create", "-R", &params.repo, "-t", &params.title];

    let body_str;
    let base_str;
    let head_str;

    if let Some(ref body) = params.body {
        body_str = body.clone();
        args.extend(["-b", &body_str]);
    }
    if let Some(ref base) = params.base {
        base_str = base.clone();
        args.extend(["-B", &base_str]);
    }
    if let Some(ref head) = params.head {
        head_str = head.clone();
        args.extend(["-H", &head_str]);
    }
    if params.draft == Some(true) {
        args.push("-d");
    }

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Merge a pull request
pub async fn pr_merge(params: PrMergeParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let mut args = vec!["pr", "merge", &number_str, "-R", &params.repo];

    let method_str;

    if let Some(ref method) = params.method {
        method_str = format!("--{}", method);
        args.push(&method_str);
    }
    if params.delete_branch == Some(true) {
        args.push("-d");
    }

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!("PR #{} merged successfully", params.number)
    } else {
        output
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Get the diff for a pull request
pub async fn pr_diff(params: PrDiffParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let mut args = vec!["pr", "diff", &number_str, "-R", &params.repo];

    if params.name_only == Some(true) {
        args.push("--name-only");
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Get CI/CD check status for a pull request
pub async fn pr_checks(params: PrChecksParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let mut args = vec!["pr", "checks", &number_str, "-R", &params.repo];

    if params.failed == Some(true) {
        args.push("--fail");
    }

    // Use execute_gh_raw_with_exit_code because gh pr checks returns:
    // - exit code 0: all checks passed
    // - exit code 1: some checks failed (stdout still has valid check data!)
    // - exit code 2+: actual errors
    match execute_gh_raw_with_exit_code(&args).await {
        Ok((output, exit_code)) => {
            // Exit code 0 or 1 both have valid output
            if output.trim().is_empty() {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "No checks reported for PR #{}",
                    params.number
                ))]))
            } else {
                // Add status indicator based on exit code
                let status_msg = if exit_code == 0 {
                    "✓ All checks passed"
                } else {
                    "✗ Some checks failed"
                };
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "{}\n\n{}",
                    status_msg, output
                ))]))
            }
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("no checks reported") {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "No checks reported for PR #{}",
                    params.number
                ))]))
            } else {
                Err(gh_to_mcp_error(e))
            }
        }
    }
}

/// Add a comment to a pull request
pub async fn pr_comment(params: PrCommentParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let args = vec![
        "pr",
        "comment",
        &number_str,
        "-R",
        &params.repo,
        "-b",
        &params.body,
    ];

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!("Comment added to PR #{}", params.number)
    } else {
        output
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Show status of your PRs in a repository
pub async fn pr_status(params: PrStatusParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["pr", "status"];

    let repo_str;
    if let Some(ref repo) = params.repo {
        repo_str = repo.clone();
        args.extend(["-R", &repo_str]);
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Submit a review on a pull request
pub async fn pr_review(params: PrReviewParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let mut args = vec!["pr", "review", &number_str, "-R", &params.repo];

    // Add the review action
    let action_flag = format!("--{}", params.action);
    args.push(&action_flag);

    let body_str;
    if let Some(ref body) = params.body {
        body_str = body.clone();
        args.extend(["-b", &body_str]);
    }

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!(
            "Review '{}' submitted for PR #{}",
            params.action, params.number
        )
    } else {
        output
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Mark a draft pull request as ready for review
pub async fn pr_ready(params: PrReadyParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let args = vec!["pr", "ready", &number_str, "-R", &params.repo];

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!("PR #{} marked as ready for review", params.number)
    } else {
        output
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// Edit a pull request
pub async fn pr_edit(params: PrEditParams) -> Result<CallToolResult, McpError> {
    let number_str = params.number.to_string();
    let mut args = vec!["pr", "edit", &number_str, "-R", &params.repo];

    let title_str;
    let body_str;
    let base_str;
    let add_labels_str;
    let remove_labels_str;
    let add_assignees_str;
    let add_reviewers_str;

    if let Some(ref title) = params.title {
        title_str = title.clone();
        args.extend(["-t", &title_str]);
    }
    if let Some(ref body) = params.body {
        body_str = body.clone();
        args.extend(["-b", &body_str]);
    }
    if let Some(ref base) = params.base {
        base_str = base.clone();
        args.extend(["-B", &base_str]);
    }
    if let Some(ref add_labels) = params.add_labels {
        add_labels_str = add_labels.clone();
        args.extend(["--add-label", &add_labels_str]);
    }
    if let Some(ref remove_labels) = params.remove_labels {
        remove_labels_str = remove_labels.clone();
        args.extend(["--remove-label", &remove_labels_str]);
    }
    if let Some(ref add_assignees) = params.add_assignees {
        add_assignees_str = add_assignees.clone();
        args.extend(["--add-assignee", &add_assignees_str]);
    }
    if let Some(ref add_reviewers) = params.add_reviewers {
        add_reviewers_str = add_reviewers.clone();
        args.extend(["--add-reviewer", &add_reviewers_str]);
    }

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!("PR #{} updated successfully", params.number)
    } else {
        output
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}
