//! MCP Server implementation
//!
//! This module defines the main MCP server that exposes GitHub CLI
//! operations as tools.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::gh::{execute_gh_action, execute_gh_json, execute_gh_raw, execute_gh_raw_with_exit_code, GhError};
use crate::types::{Issue, PullRequest, Repository, Workflow, WorkflowRun};

/// The main GitHub MCP Server
///
/// This server wraps the GitHub CLI (gh) to provide MCP-compatible tools
/// for interacting with GitHub repositories, issues, pull requests,
/// workflows, and more.
#[derive(Clone)]
pub struct GitHubMcpServer {
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue state filter (open, closed, all)")]
    pub state: Option<String>,
    #[schemars(description = "Filter by assignee username")]
    pub assignee: Option<String>,
    #[schemars(description = "Filter by label name")]
    pub label: Option<String>,
    #[schemars(description = "Maximum number of issues to return (default: 30)")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueViewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue number")]
    pub number: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueCreateParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue title")]
    pub title: String,
    #[schemars(description = "Issue body in markdown")]
    pub body: Option<String>,
    #[schemars(description = "Assignee username (@me for self)")]
    pub assignee: Option<String>,
    #[schemars(description = "Labels to add (comma-separated)")]
    pub labels: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueEditParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue number")]
    pub number: u32,
    #[schemars(description = "New issue title")]
    pub title: Option<String>,
    #[schemars(description = "New issue body")]
    pub body: Option<String>,
    #[schemars(description = "Labels to add (comma-separated)")]
    pub add_labels: Option<String>,
    #[schemars(description = "Labels to remove (comma-separated)")]
    pub remove_labels: Option<String>,
    #[schemars(description = "Assignee to add")]
    pub add_assignee: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueCloseParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue number")]
    pub number: u32,
    #[schemars(description = "Close reason (completed, not_planned)")]
    pub reason: Option<String>,
    #[schemars(description = "Comment to add when closing")]
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "PR state filter (open, closed, merged, all)")]
    pub state: Option<String>,
    #[schemars(description = "Filter by base branch")]
    pub base: Option<String>,
    #[schemars(description = "Filter by head branch (user:branch)")]
    pub head: Option<String>,
    #[schemars(description = "Filter by label")]
    pub label: Option<String>,
    #[schemars(description = "Maximum number of PRs to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrViewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrCreateParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request title")]
    pub title: String,
    #[schemars(description = "Pull request body in markdown")]
    pub body: Option<String>,
    #[schemars(description = "Base branch to merge into")]
    pub base: Option<String>,
    #[schemars(description = "Head branch with changes")]
    pub head: Option<String>,
    #[schemars(description = "Create as draft PR")]
    pub draft: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrMergeParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "Merge method (merge, squash, rebase)")]
    pub method: Option<String>,
    #[schemars(description = "Delete branch after merge")]
    pub delete_branch: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RepoListParams {
    #[schemars(description = "Filter by owner/organization")]
    pub owner: Option<String>,
    #[schemars(description = "Visibility filter (public, private, internal)")]
    pub visibility: Option<String>,
    #[schemars(description = "Filter by primary language")]
    pub language: Option<String>,
    #[schemars(description = "Maximum number of repos to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RepoViewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Include all workflows (including disabled)")]
    pub all: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowRunParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Workflow ID or filename")]
    pub workflow: String,
    #[schemars(description = "Git ref (branch/tag) to run on")]
    pub ref_name: Option<String>,
    #[schemars(description = "JSON object of workflow inputs")]
    pub inputs: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Filter by workflow ID or filename")]
    pub workflow: Option<String>,
    #[schemars(description = "Filter by branch")]
    pub branch: Option<String>,
    #[schemars(description = "Filter by status (queued, in_progress, completed)")]
    pub status: Option<String>,
    #[schemars(description = "Maximum number of runs to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunViewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Workflow run ID")]
    pub run_id: u64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunCancelParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Workflow run ID")]
    pub run_id: u64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunLogParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Workflow run ID")]
    pub run_id: u64,
    #[schemars(description = "View logs for a specific job ID only")]
    pub job: Option<String>,
    #[schemars(description = "Only show logs from failed steps (default: false shows all logs)")]
    pub failed_only: Option<bool>,
    #[schemars(description = "The attempt number of the workflow run")]
    pub attempt: Option<u32>,
}

// ============================================================================
// Phase 4: Analysis Tools Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrDiffParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "Only show names of changed files")]
    pub name_only: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrChecksParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "Only show failed checks")]
    pub failed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchPrsParams {
    #[schemars(description = "Repository in OWNER/REPO format (optional, searches all repos if not provided)")]
    pub repo: Option<String>,
    #[schemars(description = "Search query (GitHub search syntax)")]
    pub query: String,
    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrCommentParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "Comment body in markdown")]
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueCommentParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue number")]
    pub number: u32,
    #[schemars(description = "Comment body in markdown")]
    pub body: String,
}

// ============================================================================
// New Visibility Tools Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StatusParams {
    #[schemars(description = "Filter by organization")]
    pub org: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrStatusParams {
    #[schemars(description = "Repository in OWNER/REPO format (optional, uses current repo if not specified)")]
    pub repo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueStatusParams {
    #[schemars(description = "Repository in OWNER/REPO format (optional, uses current repo if not specified)")]
    pub repo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchIssuesParams {
    #[schemars(description = "Search query (GitHub search syntax)")]
    pub query: String,
    #[schemars(description = "Repository in OWNER/REPO format (optional)")]
    pub repo: Option<String>,
    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchReposParams {
    #[schemars(description = "Search query (GitHub search syntax)")]
    pub query: String,
    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchCommitsParams {
    #[schemars(description = "Search query (GitHub search syntax)")]
    pub query: String,
    #[schemars(description = "Repository in OWNER/REPO format (optional)")]
    pub repo: Option<String>,
    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunRerunParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Workflow run ID")]
    pub run_id: u64,
    #[schemars(description = "Only rerun failed jobs")]
    pub failed: Option<bool>,
    #[schemars(description = "Enable debug logging")]
    pub debug: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrReviewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "Review action: approve, request-changes, comment")]
    pub action: String,
    #[schemars(description = "Review body/comment")]
    pub body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Maximum number of releases to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseViewParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Release tag name (e.g., v1.0.0). Use 'latest' for most recent.")]
    pub tag: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LabelListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Maximum number of labels to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CacheListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Maximum number of caches to return")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SecretListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct VariableListParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrReadyParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrEditParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Pull request number")]
    pub number: u32,
    #[schemars(description = "New PR title")]
    pub title: Option<String>,
    #[schemars(description = "New PR body")]
    pub body: Option<String>,
    #[schemars(description = "New base branch")]
    pub base: Option<String>,
    #[schemars(description = "Labels to add (comma-separated)")]
    pub add_labels: Option<String>,
    #[schemars(description = "Labels to remove (comma-separated)")]
    pub remove_labels: Option<String>,
    #[schemars(description = "Assignees to add (comma-separated)")]
    pub add_assignees: Option<String>,
    #[schemars(description = "Reviewers to add (comma-separated)")]
    pub add_reviewers: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseCreateParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Tag name for the release (e.g., v1.0.0)")]
    pub tag: String,
    #[schemars(description = "Release title")]
    pub title: Option<String>,
    #[schemars(description = "Release notes body")]
    pub notes: Option<String>,
    #[schemars(description = "Target branch or commit SHA")]
    pub target: Option<String>,
    #[schemars(description = "Create as draft release")]
    pub draft: Option<bool>,
    #[schemars(description = "Mark as prerelease")]
    pub prerelease: Option<bool>,
    #[schemars(description = "Auto-generate release notes")]
    pub generate_notes: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LabelCreateParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Label name")]
    pub name: String,
    #[schemars(description = "Label color (hex without #, e.g., 'ff0000')")]
    pub color: Option<String>,
    #[schemars(description = "Label description")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IssueDeleteParams {
    #[schemars(description = "Repository in OWNER/REPO format")]
    pub repo: String,
    #[schemars(description = "Issue number")]
    pub number: u32,
}

// ============================================================================
// Tool Router Implementation
// ============================================================================

#[tool_router]
impl GitHubMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    // ========================================================================
    // Issue Tools
    // ========================================================================

    #[tool(description = "List issues in a GitHub repository with optional filters")]
    async fn gh_issue_list(
        &self,
        Parameters(params): Parameters<IssueListParams>,
    ) -> Result<CallToolResult, McpError> {
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

        let issues: Vec<Issue> = execute_gh_json(&args, Issue::list_fields())
            .await
            .map_err(gh_to_mcp_error)?;

        let json = serde_json::to_string_pretty(&issues)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "View detailed information about a specific GitHub issue")]
    async fn gh_issue_view(
        &self,
        Parameters(params): Parameters<IssueViewParams>,
    ) -> Result<CallToolResult, McpError> {
        let number_str = params.number.to_string();
        let args = vec!["issue", "view", &number_str, "-R", &params.repo];

        let issue: Issue = execute_gh_json(&args, Issue::view_fields())
            .await
            .map_err(gh_to_mcp_error)?;

        let json = serde_json::to_string_pretty(&issue)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Create a new issue in a GitHub repository")]
    async fn gh_issue_create(
        &self,
        Parameters(params): Parameters<IssueCreateParams>,
    ) -> Result<CallToolResult, McpError> {
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
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "Edit an existing GitHub issue")]
    async fn gh_issue_edit(
        &self,
        Parameters(params): Parameters<IssueEditParams>,
    ) -> Result<CallToolResult, McpError> {
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
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    #[tool(description = "Close a GitHub issue")]
    async fn gh_issue_close(
        &self,
        Parameters(params): Parameters<IssueCloseParams>,
    ) -> Result<CallToolResult, McpError> {
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
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    // ========================================================================
    // Pull Request Tools
    // ========================================================================

    #[tool(description = "List pull requests in a GitHub repository")]
    async fn gh_pr_list(
        &self,
        Parameters(params): Parameters<PrListParams>,
    ) -> Result<CallToolResult, McpError> {
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

        let prs: Vec<PullRequest> = execute_gh_json(&args, PullRequest::list_fields())
            .await
            .map_err(gh_to_mcp_error)?;

        let json = serde_json::to_string_pretty(&prs)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "View detailed information about a pull request")]
    async fn gh_pr_view(
        &self,
        Parameters(params): Parameters<PrViewParams>,
    ) -> Result<CallToolResult, McpError> {
        let number_str = params.number.to_string();
        let args = vec!["pr", "view", &number_str, "-R", &params.repo];

        let pr: PullRequest = execute_gh_json(&args, PullRequest::view_fields())
            .await
            .map_err(gh_to_mcp_error)?;

        let json = serde_json::to_string_pretty(&pr)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Create a new pull request")]
    async fn gh_pr_create(
        &self,
        Parameters(params): Parameters<PrCreateParams>,
    ) -> Result<CallToolResult, McpError> {
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

    #[tool(description = "Merge a pull request")]
    async fn gh_pr_merge(
        &self,
        Parameters(params): Parameters<PrMergeParams>,
    ) -> Result<CallToolResult, McpError> {
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

    // ========================================================================
    // Repository Tools
    // ========================================================================

    #[tool(description = "List repositories")]
    async fn gh_repo_list(
        &self,
        Parameters(params): Parameters<RepoListParams>,
    ) -> Result<CallToolResult, McpError> {
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

        let json = serde_json::to_string_pretty(&repos)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "View repository details")]
    async fn gh_repo_view(
        &self,
        Parameters(params): Parameters<RepoViewParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = vec!["repo", "view", &params.repo];

        let repo: Repository = execute_gh_json(&args, Repository::view_fields())
            .await
            .map_err(gh_to_mcp_error)?;

        let json = serde_json::to_string_pretty(&repo)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Workflow Tools
    // ========================================================================

    #[tool(description = "List workflows in a repository")]
    async fn gh_workflow_list(
        &self,
        Parameters(params): Parameters<WorkflowListParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut args = vec!["workflow", "list", "-R", &params.repo];

        if params.all == Some(true) {
            args.push("-a");
        }

        let workflows: Vec<Workflow> = execute_gh_json(&args, Workflow::list_fields())
            .await
            .map_err(gh_to_mcp_error)?;

        let json = serde_json::to_string_pretty(&workflows)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Trigger a workflow run")]
    async fn gh_workflow_run(
        &self,
        Parameters(params): Parameters<WorkflowRunParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut args = vec!["workflow", "run", &params.workflow, "-R", &params.repo];

        let ref_str;
        let inputs_str;

        if let Some(ref ref_name) = params.ref_name {
            ref_str = ref_name.clone();
            args.extend(["--ref", &ref_str]);
        }
        if let Some(ref inputs) = params.inputs {
            inputs_str = inputs.clone();
            args.extend(["-f", &inputs_str]);
        }

        let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
        let msg = if output.is_empty() {
            format!("Workflow '{}' triggered successfully", params.workflow)
        } else {
            output
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    #[tool(description = "List workflow runs")]
    async fn gh_run_list(
        &self,
        Parameters(params): Parameters<RunListParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut args = vec!["run", "list", "-R", &params.repo];

        let workflow_str;
        let branch_str;
        let status_str;
        let limit_str;

        if let Some(ref workflow) = params.workflow {
            workflow_str = workflow.clone();
            args.extend(["-w", &workflow_str]);
        }
        if let Some(ref branch) = params.branch {
            branch_str = branch.clone();
            args.extend(["-b", &branch_str]);
        }
        if let Some(ref status) = params.status {
            status_str = status.clone();
            args.extend(["-s", &status_str]);
        }
        if let Some(limit) = params.limit {
            limit_str = limit.to_string();
            args.extend(["-L", &limit_str]);
        }

        let runs: Vec<WorkflowRun> = execute_gh_json(&args, WorkflowRun::list_fields())
            .await
            .map_err(gh_to_mcp_error)?;

        let json = serde_json::to_string_pretty(&runs)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "View workflow run details")]
    async fn gh_run_view(
        &self,
        Parameters(params): Parameters<RunViewParams>,
    ) -> Result<CallToolResult, McpError> {
        let run_id_str = params.run_id.to_string();
        let args = vec!["run", "view", &run_id_str, "-R", &params.repo];

        let run: WorkflowRun = execute_gh_json(&args, WorkflowRun::view_fields())
            .await
            .map_err(gh_to_mcp_error)?;

        let json = serde_json::to_string_pretty(&run)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Cancel a workflow run")]
    async fn gh_run_cancel(
        &self,
        Parameters(params): Parameters<RunCancelParams>,
    ) -> Result<CallToolResult, McpError> {
        let run_id_str = params.run_id.to_string();
        let args = vec!["run", "cancel", &run_id_str, "-R", &params.repo];

        let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
        let msg = if output.is_empty() {
            format!("Run {} cancelled successfully", params.run_id)
        } else {
            output
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    #[tool(description = "View logs for a workflow run or specific job. Use failed_only=true to see only failed step logs, which is useful for debugging CI failures.")]
    async fn gh_run_log(
        &self,
        Parameters(params): Parameters<RunLogParams>,
    ) -> Result<CallToolResult, McpError> {
        let run_id_str = params.run_id.to_string();
        let mut args = vec!["run", "view", &run_id_str, "-R", &params.repo];

        let job_str;
        let attempt_str;

        // Add --log or --log-failed flag
        if params.failed_only == Some(true) {
            args.push("--log-failed");
        } else {
            args.push("--log");
        }

        // Add job filter if specified
        if let Some(ref job) = params.job {
            job_str = job.clone();
            args.extend(["--job", &job_str]);
        }

        // Add attempt number if specified
        if let Some(attempt) = params.attempt {
            attempt_str = attempt.to_string();
            args.extend(["--attempt", &attempt_str]);
        }

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;

        if output.trim().is_empty() {
            let msg = if params.failed_only == Some(true) {
                format!("No failed step logs found for run {}", params.run_id)
            } else {
                format!("No logs found for run {}", params.run_id)
            };
            Ok(CallToolResult::success(vec![Content::text(msg)]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(output)]))
        }
    }

    // ========================================================================
    // Phase 4: Analysis Tools
    // ========================================================================

    #[tool(description = "Get the diff for a pull request")]
    async fn gh_pr_diff(
        &self,
        Parameters(params): Parameters<PrDiffParams>,
    ) -> Result<CallToolResult, McpError> {
        let number_str = params.number.to_string();
        let mut args = vec!["pr", "diff", &number_str, "-R", &params.repo];

        if params.name_only == Some(true) {
            args.push("--name-only");
        }

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "Get CI/CD check status for a pull request")]
    async fn gh_pr_checks(
        &self,
        Parameters(params): Parameters<PrChecksParams>,
    ) -> Result<CallToolResult, McpError> {
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
                    Ok(CallToolResult::success(vec![Content::text(
                        format!("No checks reported for PR #{}", params.number)
                    )]))
                } else {
                    // Add status indicator based on exit code
                    let status_msg = if exit_code == 0 {
                        "✓ All checks passed"
                    } else {
                        "✗ Some checks failed"
                    };
                    Ok(CallToolResult::success(vec![Content::text(
                        format!("{}\n\n{}", status_msg, output)
                    )]))
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("no checks reported") {
                    Ok(CallToolResult::success(vec![Content::text(
                        format!("No checks reported for PR #{}", params.number)
                    )]))
                } else {
                    Err(gh_to_mcp_error(e))
                }
            }
        }
    }

    #[tool(description = "Search for pull requests using GitHub search syntax")]
    async fn gh_search_prs(
        &self,
        Parameters(params): Parameters<SearchPrsParams>,
    ) -> Result<CallToolResult, McpError> {
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
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "Add a comment to a pull request")]
    async fn gh_pr_comment(
        &self,
        Parameters(params): Parameters<PrCommentParams>,
    ) -> Result<CallToolResult, McpError> {
        let number_str = params.number.to_string();
        let args = vec![
            "pr", "comment", &number_str,
            "-R", &params.repo,
            "-b", &params.body,
        ];

        let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
        let msg = if output.is_empty() {
            format!("Comment added to PR #{}", params.number)
        } else {
            output
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    #[tool(description = "Add a comment to an issue")]
    async fn gh_issue_comment(
        &self,
        Parameters(params): Parameters<IssueCommentParams>,
    ) -> Result<CallToolResult, McpError> {
        let number_str = params.number.to_string();
        let args = vec![
            "issue", "comment", &number_str,
            "-R", &params.repo,
            "-b", &params.body,
        ];

        let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
        let msg = if output.is_empty() {
            format!("Comment added to issue #{}", params.number)
        } else {
            output
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    // ========================================================================
    // Tier 1: High-Value Visibility Tools
    // ========================================================================

    #[tool(description = "Show status of relevant issues, PRs, and notifications across all repositories. Shows mentions, review requests, and assigned items.")]
    async fn gh_status(
        &self,
        Parameters(params): Parameters<StatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut args = vec!["status"];

        let org_str;
        if let Some(ref org) = params.org {
            org_str = format!("-o={}", org);
            args.push(&org_str);
        }

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "Show status of your PRs in a repository - open PRs, checks failing, approved, needs review")]
    async fn gh_pr_status(
        &self,
        Parameters(params): Parameters<PrStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut args = vec!["pr", "status"];

        let repo_str;
        if let Some(ref repo) = params.repo {
            repo_str = repo.clone();
            args.extend(["-R", &repo_str]);
        }

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "Show status of issues relevant to you - assigned, mentioned, created by you")]
    async fn gh_issue_status(
        &self,
        Parameters(params): Parameters<IssueStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut args = vec!["issue", "status"];

        let repo_str;
        if let Some(ref repo) = params.repo {
            repo_str = repo.clone();
            args.extend(["-R", &repo_str]);
        }

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "Search for issues using GitHub search syntax")]
    async fn gh_search_issues(
        &self,
        Parameters(params): Parameters<SearchIssuesParams>,
    ) -> Result<CallToolResult, McpError> {
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
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "Search for repositories using GitHub search syntax")]
    async fn gh_search_repos(
        &self,
        Parameters(params): Parameters<SearchReposParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut args = vec!["search", "repos", &params.query];

        let limit_str;
        if let Some(limit) = params.limit {
            limit_str = limit.to_string();
            args.extend(["--limit", &limit_str]);
        }

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "Search for commits using GitHub search syntax")]
    async fn gh_search_commits(
        &self,
        Parameters(params): Parameters<SearchCommitsParams>,
    ) -> Result<CallToolResult, McpError> {
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
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "Rerun a workflow run - optionally only failed jobs")]
    async fn gh_run_rerun(
        &self,
        Parameters(params): Parameters<RunRerunParams>,
    ) -> Result<CallToolResult, McpError> {
        let run_id_str = params.run_id.to_string();
        let mut args = vec!["run", "rerun", &run_id_str, "-R", &params.repo];

        if params.failed == Some(true) {
            args.push("--failed");
        }
        if params.debug == Some(true) {
            args.push("--debug");
        }

        let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
        let msg = if output.is_empty() {
            if params.failed == Some(true) {
                format!("Rerun triggered for failed jobs of run {}", params.run_id)
            } else {
                format!("Rerun triggered for all jobs of run {}", params.run_id)
            }
        } else {
            output
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    #[tool(description = "Submit a review on a pull request - approve, request-changes, or comment")]
    async fn gh_pr_review(
        &self,
        Parameters(params): Parameters<PrReviewParams>,
    ) -> Result<CallToolResult, McpError> {
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
            format!("Review '{}' submitted for PR #{}", params.action, params.number)
        } else {
            output
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    // ========================================================================
    // Tier 2: Release Tools
    // ========================================================================

    #[tool(description = "List releases in a repository")]
    async fn gh_release_list(
        &self,
        Parameters(params): Parameters<ReleaseListParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut args = vec!["release", "list", "-R", &params.repo];

        let limit_str;
        if let Some(limit) = params.limit {
            limit_str = limit.to_string();
            args.extend(["-L", &limit_str]);
        }

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "View release details including changelog and assets. Use 'latest' for the most recent release.")]
    async fn gh_release_view(
        &self,
        Parameters(params): Parameters<ReleaseViewParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = vec!["release", "view", &params.tag, "-R", &params.repo];

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    // ========================================================================
    // Tier 3: Repository Resource Tools
    // ========================================================================

    #[tool(description = "List labels in a repository")]
    async fn gh_label_list(
        &self,
        Parameters(params): Parameters<LabelListParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut args = vec!["label", "list", "-R", &params.repo];

        let limit_str;
        if let Some(limit) = params.limit {
            limit_str = limit.to_string();
            args.extend(["-L", &limit_str]);
        }

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "List GitHub Actions caches in a repository")]
    async fn gh_cache_list(
        &self,
        Parameters(params): Parameters<CacheListParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut args = vec!["cache", "list", "-R", &params.repo];

        let limit_str;
        if let Some(limit) = params.limit {
            limit_str = limit.to_string();
            args.extend(["-L", &limit_str]);
        }

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "List repository secrets (names only, not values)")]
    async fn gh_secret_list(
        &self,
        Parameters(params): Parameters<SecretListParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = vec!["secret", "list", "-R", &params.repo];

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "List repository variables for GitHub Actions")]
    async fn gh_variable_list(
        &self,
        Parameters(params): Parameters<VariableListParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = vec!["variable", "list", "-R", &params.repo];

        let output = execute_gh_raw(&args).await.map_err(gh_to_mcp_error)?;
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    // ========================================================================
    // Tier 4: Additional PR/Issue Operations
    // ========================================================================

    #[tool(description = "Mark a draft pull request as ready for review")]
    async fn gh_pr_ready(
        &self,
        Parameters(params): Parameters<PrReadyParams>,
    ) -> Result<CallToolResult, McpError> {
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

    #[tool(description = "Edit a pull request - change title, body, labels, assignees, reviewers")]
    async fn gh_pr_edit(
        &self,
        Parameters(params): Parameters<PrEditParams>,
    ) -> Result<CallToolResult, McpError> {
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

    // ========================================================================
    // Create/Write Operations (last tier)
    // ========================================================================

    #[tool(description = "Create a new release in a repository")]
    async fn gh_release_create(
        &self,
        Parameters(params): Parameters<ReleaseCreateParams>,
    ) -> Result<CallToolResult, McpError> {
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

    #[tool(description = "Create a new label in a repository")]
    async fn gh_label_create(
        &self,
        Parameters(params): Parameters<LabelCreateParams>,
    ) -> Result<CallToolResult, McpError> {
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

    #[tool(description = "Delete an issue (requires admin permissions)")]
    async fn gh_issue_delete(
        &self,
        Parameters(params): Parameters<IssueDeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        let number_str = params.number.to_string();
        let args = vec!["issue", "delete", &number_str, "-R", &params.repo, "--yes"];

        let output = execute_gh_action(&args).await.map_err(gh_to_mcp_error)?;
        let msg = if output.is_empty() {
            format!("Issue #{} deleted successfully", params.number)
        } else {
            output
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for GitHubMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "GitHub CLI MCP Server - provides tools for interacting with GitHub \
                 repositories, issues, pull requests, and workflows using the gh CLI. \
                 Requires gh to be installed and authenticated."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a GhError to an MCP error
fn gh_to_mcp_error(e: GhError) -> McpError {
    McpError::internal_error(e.to_string(), None)
}
