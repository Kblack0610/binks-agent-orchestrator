//! MCP Server implementation
//!
//! This module defines the main MCP server that exposes GitHub CLI
//! operations as tools. Handler implementations are in the handlers/ module.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};

use crate::handlers;
use crate::params::*;

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
// Tool Router - Each tool delegates to its handler
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
        handlers::issue_list(params).await
    }

    #[tool(description = "View detailed information about a specific GitHub issue")]
    async fn gh_issue_view(
        &self,
        Parameters(params): Parameters<IssueViewParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_view(params).await
    }

    #[tool(description = "Create a new issue in a GitHub repository")]
    async fn gh_issue_create(
        &self,
        Parameters(params): Parameters<IssueCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_create(params).await
    }

    #[tool(description = "Edit an existing GitHub issue")]
    async fn gh_issue_edit(
        &self,
        Parameters(params): Parameters<IssueEditParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_edit(params).await
    }

    #[tool(description = "Close a GitHub issue")]
    async fn gh_issue_close(
        &self,
        Parameters(params): Parameters<IssueCloseParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_close(params).await
    }

    #[tool(description = "Add a comment to an issue")]
    async fn gh_issue_comment(
        &self,
        Parameters(params): Parameters<IssueCommentParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_comment(params).await
    }

    #[tool(description = "Delete an issue (requires admin permissions)")]
    async fn gh_issue_delete(
        &self,
        Parameters(params): Parameters<IssueDeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_delete(params).await
    }

    #[tool(description = "Show status of issues relevant to you - assigned, mentioned, created by you")]
    async fn gh_issue_status(
        &self,
        Parameters(params): Parameters<IssueStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::issue_status(params).await
    }

    // ========================================================================
    // Pull Request Tools
    // ========================================================================

    #[tool(description = "List pull requests in a GitHub repository")]
    async fn gh_pr_list(
        &self,
        Parameters(params): Parameters<PrListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::pr_list(params).await
    }

    #[tool(description = "View detailed information about a pull request")]
    async fn gh_pr_view(
        &self,
        Parameters(params): Parameters<PrViewParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::pr_view(params).await
    }

    #[tool(description = "Create a new pull request")]
    async fn gh_pr_create(
        &self,
        Parameters(params): Parameters<PrCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::pr_create(params).await
    }

    #[tool(description = "Merge a pull request")]
    async fn gh_pr_merge(
        &self,
        Parameters(params): Parameters<PrMergeParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::pr_merge(params).await
    }

    #[tool(description = "Get the diff for a pull request")]
    async fn gh_pr_diff(
        &self,
        Parameters(params): Parameters<PrDiffParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::pr_diff(params).await
    }

    #[tool(description = "Get CI/CD check status for a pull request")]
    async fn gh_pr_checks(
        &self,
        Parameters(params): Parameters<PrChecksParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::pr_checks(params).await
    }

    #[tool(description = "Add a comment to a pull request")]
    async fn gh_pr_comment(
        &self,
        Parameters(params): Parameters<PrCommentParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::pr_comment(params).await
    }

    #[tool(description = "Show status of your PRs in a repository - open PRs, checks failing, approved, needs review")]
    async fn gh_pr_status(
        &self,
        Parameters(params): Parameters<PrStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::pr_status(params).await
    }

    #[tool(description = "Submit a review on a pull request - approve, request-changes, or comment")]
    async fn gh_pr_review(
        &self,
        Parameters(params): Parameters<PrReviewParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::pr_review(params).await
    }

    #[tool(description = "Mark a draft pull request as ready for review")]
    async fn gh_pr_ready(
        &self,
        Parameters(params): Parameters<PrReadyParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::pr_ready(params).await
    }

    #[tool(description = "Edit a pull request - change title, body, labels, assignees, reviewers")]
    async fn gh_pr_edit(
        &self,
        Parameters(params): Parameters<PrEditParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::pr_edit(params).await
    }

    // ========================================================================
    // Repository Tools
    // ========================================================================

    #[tool(description = "List repositories")]
    async fn gh_repo_list(
        &self,
        Parameters(params): Parameters<RepoListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::repo_list(params).await
    }

    #[tool(description = "View repository details")]
    async fn gh_repo_view(
        &self,
        Parameters(params): Parameters<RepoViewParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::repo_view(params).await
    }

    // ========================================================================
    // Workflow Tools
    // ========================================================================

    #[tool(description = "List workflows in a repository")]
    async fn gh_workflow_list(
        &self,
        Parameters(params): Parameters<WorkflowListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::workflow_list(params).await
    }

    #[tool(description = "Trigger a workflow run")]
    async fn gh_workflow_run(
        &self,
        Parameters(params): Parameters<WorkflowRunParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::workflow_run(params).await
    }

    #[tool(description = "List workflow runs")]
    async fn gh_run_list(
        &self,
        Parameters(params): Parameters<RunListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::run_list(params).await
    }

    #[tool(description = "View workflow run details")]
    async fn gh_run_view(
        &self,
        Parameters(params): Parameters<RunViewParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::run_view(params).await
    }

    #[tool(description = "Cancel a workflow run")]
    async fn gh_run_cancel(
        &self,
        Parameters(params): Parameters<RunCancelParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::run_cancel(params).await
    }

    #[tool(description = "View logs for a workflow run or specific job")]
    async fn gh_run_log(
        &self,
        Parameters(params): Parameters<RunLogParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::run_log(params).await
    }

    #[tool(description = "Rerun a workflow run - optionally only failed jobs")]
    async fn gh_run_rerun(
        &self,
        Parameters(params): Parameters<RunRerunParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::run_rerun(params).await
    }

    // ========================================================================
    // Search Tools
    // ========================================================================

    #[tool(description = "Show status of relevant issues, PRs, and notifications across all repositories. Shows mentions, review requests, and assigned items.")]
    async fn gh_status(
        &self,
        Parameters(params): Parameters<StatusParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::status(params).await
    }

    #[tool(description = "Search for pull requests using GitHub search syntax")]
    async fn gh_search_prs(
        &self,
        Parameters(params): Parameters<SearchPrsParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::search_prs(params).await
    }

    #[tool(description = "Search for issues using GitHub search syntax")]
    async fn gh_search_issues(
        &self,
        Parameters(params): Parameters<SearchIssuesParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::search_issues(params).await
    }

    #[tool(description = "Search for repositories using GitHub search syntax")]
    async fn gh_search_repos(
        &self,
        Parameters(params): Parameters<SearchReposParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::search_repos(params).await
    }

    #[tool(description = "Search for commits using GitHub search syntax")]
    async fn gh_search_commits(
        &self,
        Parameters(params): Parameters<SearchCommitsParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::search_commits(params).await
    }

    // ========================================================================
    // Release Tools
    // ========================================================================

    #[tool(description = "List releases in a repository")]
    async fn gh_release_list(
        &self,
        Parameters(params): Parameters<ReleaseListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::release_list(params).await
    }

    #[tool(description = "View release details including changelog and assets. Use 'latest' for the most recent release.")]
    async fn gh_release_view(
        &self,
        Parameters(params): Parameters<ReleaseViewParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::release_view(params).await
    }

    #[tool(description = "Create a new release in a repository")]
    async fn gh_release_create(
        &self,
        Parameters(params): Parameters<ReleaseCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::release_create(params).await
    }

    // ========================================================================
    // Label Tools
    // ========================================================================

    #[tool(description = "List labels in a repository")]
    async fn gh_label_list(
        &self,
        Parameters(params): Parameters<LabelListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::label_list(params).await
    }

    #[tool(description = "Create a new label in a repository")]
    async fn gh_label_create(
        &self,
        Parameters(params): Parameters<LabelCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::label_create(params).await
    }

    // ========================================================================
    // Repository Resource Tools
    // ========================================================================

    #[tool(description = "List GitHub Actions caches in a repository")]
    async fn gh_cache_list(
        &self,
        Parameters(params): Parameters<CacheListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::cache_list(params).await
    }

    #[tool(description = "List repository secrets (names only, not values)")]
    async fn gh_secret_list(
        &self,
        Parameters(params): Parameters<SecretListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::secret_list(params).await
    }

    #[tool(description = "List repository variables for GitHub Actions")]
    async fn gh_variable_list(
        &self,
        Parameters(params): Parameters<VariableListParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::variable_list(params).await
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

impl Default for GitHubMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
