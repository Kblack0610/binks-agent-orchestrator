//! Repository handler implementations

use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;

use crate::gh::execute_gh_json;
use crate::params::{RepoListParams, RepoViewParams};
use crate::types::Repository;

use super::gh_to_mcp_error;

/// List repositories
pub async fn repo_list(params: RepoListParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["repo", "list"];

    let owner_str;
    let visibility_str;
    let language_str;
    let limit_str;

    if let Some(ref owner) = params.owner {
        owner_str = owner.clone();
        args.push(&owner_str);
    }
    if let Some(ref visibility) = params.visibility {
        visibility_str = visibility.clone();
        args.extend(["--visibility", &visibility_str]);
    }
    if let Some(ref language) = params.language {
        language_str = language.clone();
        args.extend(["-l", &language_str]);
    }
    if let Some(limit) = params.limit {
        limit_str = limit.to_string();
        args.extend(["-L", &limit_str]);
    }

    let repos: Vec<Repository> = execute_gh_json(&args, Repository::list_fields())
        .await
        .map_err(gh_to_mcp_error)?;

    let json = serde_json::to_string(&repos)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// View repository details
pub async fn repo_view(params: RepoViewParams) -> Result<CallToolResult, McpError> {
    let args = vec!["repo", "view", &params.repo];

    let repo: Repository = execute_gh_json(&args, Repository::view_fields())
        .await
        .map_err(gh_to_mcp_error)?;

    let json = serde_json::to_string(&repo)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}
