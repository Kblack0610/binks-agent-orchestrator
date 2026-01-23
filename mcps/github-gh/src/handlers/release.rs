//! Release, label, cache, secret, and variable handler implementations

use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;

use crate::gh::{execute_gh_action, execute_gh_raw};
use crate::params::{
    CacheListParams, LabelCreateParams, LabelListParams, ReleaseCreateParams, ReleaseListParams,
    ReleaseViewParams, SecretListParams, VariableListParams,
};

use super::gh_to_mcp_error;

/// List releases in a repository
pub async fn release_list(params: ReleaseListParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["release", "list", "-R", &params.repo];

    let limit_str;
    if let Some(limit) = params.limit {
        limit_str = limit.to_string();
        args.extend(["-L", &limit_str]);
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// View release details including changelog and assets
pub async fn release_view(params: ReleaseViewParams) -> Result<CallToolResult, McpError> {
    let args = vec!["release", "view", &params.tag, "-R", &params.repo];

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Create a new release in a repository
pub async fn release_create(params: ReleaseCreateParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["release", "create", &params.tag, "-R", &params.repo];

    let title_str;
    let notes_str;
    let target_str;

    if let Some(ref title) = params.title {
        title_str = title.clone();
        args.extend(["-t", &title_str]);
    }
    if let Some(ref notes) = params.notes {
        notes_str = notes.clone();
        args.extend(["-n", &notes_str]);
    }
    if let Some(ref target) = params.target {
        target_str = target.clone();
        args.extend(["--target", &target_str]);
    }
    if params.draft == Some(true) {
        args.push("--draft");
    }
    if params.prerelease == Some(true) {
        args.push("--prerelease");
    }
    if params.generate_notes == Some(true) {
        args.push("--generate-notes");
    }

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!("Release {} created successfully", params.tag)
    } else {
        output
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// List labels in a repository
pub async fn label_list(params: LabelListParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["label", "list", "-R", &params.repo];

    let limit_str;
    if let Some(limit) = params.limit {
        limit_str = limit.to_string();
        args.extend(["-L", &limit_str]);
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Create a new label in a repository
pub async fn label_create(params: LabelCreateParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["label", "create", &params.name, "-R", &params.repo];

    let color_str;
    let description_str;

    if let Some(ref color) = params.color {
        color_str = color.clone();
        args.extend(["-c", &color_str]);
    }
    if let Some(ref description) = params.description {
        description_str = description.clone();
        args.extend(["-d", &description_str]);
    }

    let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
    let msg = if output.is_empty() {
        format!("Label '{}' created successfully", params.name)
    } else {
        output
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

/// List GitHub Actions caches in a repository
pub async fn cache_list(params: CacheListParams) -> Result<CallToolResult, McpError> {
    let mut args = vec!["cache", "list", "-R", &params.repo];

    let limit_str;
    if let Some(limit) = params.limit {
        limit_str = limit.to_string();
        args.extend(["-L", &limit_str]);
    }

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// List repository secrets (names only, not values)
pub async fn secret_list(params: SecretListParams) -> Result<CallToolResult, McpError> {
    let args = vec!["secret", "list", "-R", &params.repo];

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// List repository variables for GitHub Actions
pub async fn variable_list(params: VariableListParams) -> Result<CallToolResult, McpError> {
    let args = vec!["variable", "list", "-R", &params.repo];

    let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
    Ok(CallToolResult::success(vec![Content::text(output)]))
}
