//! Search and status handler implementations

use mcp_common::{text_success, CallToolResult, McpError};

use crate::gh::execute_gh_raw;
use crate::params::{
    SearchCommitsParams, SearchIssuesParams, SearchPrsParams, SearchReposParams, StatusParams,
};

use super::gh_to_mcp_error;

/// Show status of relevant issues, PRs, and notifications across all repositories
pub async fn status(params: StatusParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["status"];

    let org_str;
    if let Some(ref org) = params.org {
        org_str = format!("-o={}", org);
        args.push(&org_str);
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(text_success(output))
}

/// Search for pull requests using GitHub search syntax
pub async fn search_prs(params: SearchPrsParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["search", "prs", &params.query];

    let repo_str;
    let limit_str;

    if let Some(ref repo) = params.repo {
        repo_str = format!("--repo={}", repo);
        args.push(&repo_str);
    }
    if let Some(limit) = params.limit {
        limit_str = limit.to_string();
        args.extend(["--limit", &limit_str]);
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(text_success(output))
}

/// Search for issues using GitHub search syntax
pub async fn search_issues(params: SearchIssuesParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["search", "issues", &params.query];

    let repo_str;
    let limit_str;

    if let Some(ref repo) = params.repo {
        repo_str = format!("--repo={}", repo);
        args.push(&repo_str);
    }
    if let Some(limit) = params.limit {
        limit_str = limit.to_string();
        args.extend(["--limit", &limit_str]);
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(text_success(output))
}

/// Search for repositories using GitHub search syntax
pub async fn search_repos(params: SearchReposParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["search", "repos", &params.query];

    let limit_str;
    if let Some(limit) = params.limit {
        limit_str = limit.to_string();
        args.extend(["--limit", &limit_str]);
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(text_success(output))
}

/// Search for commits using GitHub search syntax
pub async fn search_commits(params: SearchCommitsParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["search", "commits", &params.query];

    let repo_str;
    let limit_str;

    if let Some(ref repo) = params.repo {
        repo_str = format!("--repo={}", repo);
        args.push(&repo_str);
    }
    if let Some(limit) = params.limit {
        limit_str = limit.to_string();
        args.extend(["--limit", &limit_str]);
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(text_success(output))
}
