//! MCP Server implementation for Git operations

use chrono::{DateTime, TimeZone, Utc};
use git2::{BlameOptions, BranchType, Delta, DiffOptions, Repository, StatusOptions};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;

use crate::types::*;

/// The Git MCP Server
#[derive(Clone)]
pub struct GitMcpServer {
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Parameter Types
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatusParams {
    /// Path to the git repository (defaults to current directory)
    #[serde(default)]
    pub repo_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LogParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// Maximum number of commits to return (default: 10)
    #[serde(default)]
    pub limit: Option<usize>,
    /// Reference to start from (branch, tag, or commit)
    #[serde(default)]
    pub rev: Option<String>,
    /// Path filter (only commits affecting this path)
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiffParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// From reference (defaults to HEAD)
    #[serde(default)]
    pub from_ref: Option<String>,
    /// To reference (defaults to working directory)
    #[serde(default)]
    pub to_ref: Option<String>,
    /// Include file contents in diff (default: true)
    #[serde(default = "default_true")]
    pub include_patch: Option<bool>,
    /// Path filter
    #[serde(default)]
    pub path: Option<String>,
}

fn default_true() -> Option<bool> {
    Some(true)
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// Reference to show (commit, tag, etc.)
    pub rev: String,
    /// Include diff in output (default: false)
    #[serde(default)]
    pub include_diff: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BranchListParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// Include remote branches (default: false)
    #[serde(default)]
    pub include_remote: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlameParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// Path to the file to blame
    pub file_path: String,
    /// Starting line (1-indexed, default: 1)
    #[serde(default)]
    pub start_line: Option<usize>,
    /// Ending line (default: end of file)
    #[serde(default)]
    pub end_line: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StashParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
    /// Action: "list", "show", "save", "pop", "apply", "drop"
    pub action: String,
    /// Stash index for show/pop/apply/drop (default: 0)
    #[serde(default)]
    pub index: Option<usize>,
    /// Message for save action
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoteListParams {
    /// Path to the git repository
    #[serde(default)]
    pub repo_path: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn git_error_to_mcp(e: GitError) -> McpError {
    match &e {
        GitError::RepoNotFound(_) | GitError::RefNotFound(_) | GitError::FileNotFound(_) => {
            McpError::invalid_params(e.to_string(), None)
        }
        GitError::InvalidRef(_) | GitError::InvalidPath(_) => {
            McpError::invalid_request(e.to_string(), None)
        }
        _ => McpError::internal_error(e.to_string(), None),
    }
}

fn open_repo(path: Option<&str>) -> GitResult<Repository> {
    let repo_path = path.unwrap_or(".");
    Repository::discover(repo_path).map_err(|_| GitError::RepoNotFound(repo_path.to_string()))
}

fn git_time_to_datetime(time: git2::Time) -> DateTime<Utc> {
    Utc.timestamp_opt(time.seconds(), 0)
        .single()
        .unwrap_or_else(Utc::now)
}

fn commit_to_info(commit: &git2::Commit) -> CommitInfo {
    CommitInfo {
        id: commit.id().to_string(),
        short_id: commit.id().to_string()[..7].to_string(),
        message: commit.message().unwrap_or("").to_string(),
        author_name: commit.author().name().unwrap_or("").to_string(),
        author_email: commit.author().email().unwrap_or("").to_string(),
        author_time: git_time_to_datetime(commit.author().when()),
        committer_name: commit.committer().name().unwrap_or("").to_string(),
        committer_email: commit.committer().email().unwrap_or("").to_string(),
        committer_time: git_time_to_datetime(commit.committer().when()),
        parent_ids: commit.parent_ids().map(|id| id.to_string()).collect(),
    }
}

fn delta_to_status(delta: Delta) -> &'static str {
    match delta {
        Delta::Added => "new",
        Delta::Deleted => "deleted",
        Delta::Modified => "modified",
        Delta::Renamed => "renamed",
        Delta::Copied => "copied",
        Delta::Typechange => "typechange",
        _ => "unknown",
    }
}

// ============================================================================
// Server Implementation
// ============================================================================

#[tool_router]
impl GitMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get the status of a git repository, including staged, modified, and untracked files")]
    async fn git_status(
        &self,
        Parameters(params): Parameters<StatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;

        let mut status_opts = StatusOptions::new();
        status_opts
            .include_untracked(true)
            .include_ignored(false)
            .include_unmodified(false);

        let statuses = repo.statuses(Some(&mut status_opts)).map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

        let head = repo.head().ok();
        let branch = head.as_ref().and_then(|h| h.shorthand().map(String::from));
        let head_commit = head.and_then(|h| h.target()).map(|id| id.to_string());

        let mut staged = Vec::new();
        let mut modified = Vec::new();
        let mut untracked = Vec::new();
        let mut conflicted = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = entry.status();

            if status.is_conflicted() {
                conflicted.push(path);
            } else if status.is_wt_new() {
                untracked.push(path);
            } else {
                // Index changes (staged)
                if status.is_index_new() {
                    staged.push(FileStatus {
                        path: path.clone(),
                        status: "new".to_string(),
                        old_path: None,
                    });
                } else if status.is_index_modified() {
                    staged.push(FileStatus {
                        path: path.clone(),
                        status: "modified".to_string(),
                        old_path: None,
                    });
                } else if status.is_index_deleted() {
                    staged.push(FileStatus {
                        path: path.clone(),
                        status: "deleted".to_string(),
                        old_path: None,
                    });
                } else if status.is_index_renamed() {
                    staged.push(FileStatus {
                        path: path.clone(),
                        status: "renamed".to_string(),
                        old_path: entry.head_to_index().and_then(|d| d.old_file().path().map(|p| p.to_string_lossy().to_string())),
                    });
                }

                // Working tree changes (modified)
                if status.is_wt_modified() {
                    modified.push(FileStatus {
                        path: path.clone(),
                        status: "modified".to_string(),
                        old_path: None,
                    });
                } else if status.is_wt_deleted() {
                    modified.push(FileStatus {
                        path: path.clone(),
                        status: "deleted".to_string(),
                        old_path: None,
                    });
                } else if status.is_wt_renamed() {
                    modified.push(FileStatus {
                        path: path.clone(),
                        status: "renamed".to_string(),
                        old_path: entry.index_to_workdir().and_then(|d| d.old_file().path().map(|p| p.to_string_lossy().to_string())),
                    });
                }
            }
        }

        let is_clean = staged.is_empty() && modified.is_empty() && untracked.is_empty() && conflicted.is_empty();

        let response = StatusResponse {
            repo_path: repo.path().parent().unwrap_or(Path::new(".")).to_string_lossy().to_string(),
            branch,
            head_commit,
            is_clean,
            staged,
            modified,
            untracked,
            conflicted,
        };

        Ok(CallToolResult::success(vec![Content::json(&response)?]))
    }

    #[tool(description = "Get the commit log of a git repository")]
    async fn git_log(
        &self,
        Parameters(params): Parameters<LogParams>,
    ) -> Result<CallToolResult, McpError> {
        let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;
        let limit = params.limit.unwrap_or(10);

        let mut revwalk = repo.revwalk().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

        // Set starting point
        if let Some(rev) = &params.rev {
            let obj = repo.revparse_single(rev).map_err(|_| git_error_to_mcp(GitError::RefNotFound(rev.clone())))?;
            revwalk.push(obj.id()).map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
        } else {
            revwalk.push_head().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
        }

        revwalk.set_sorting(git2::Sort::TIME).map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

        let mut commits = Vec::new();
        for (count, oid_result) in revwalk.enumerate() {
            if count >= limit {
                break;
            }

            let oid = oid_result.map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
            let commit = repo.find_commit(oid).map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

            // Path filter if specified
            if let Some(filter_path) = &params.path {
                // Check if commit touches the path
                let tree = commit.tree().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
                let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

                let mut diff_opts = DiffOptions::new();
                diff_opts.pathspec(filter_path);

                let diff = repo
                    .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))
                    .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

                if diff.deltas().count() == 0 {
                    continue; // Skip commits that don't touch the path
                }
            }

            commits.push(commit_to_info(&commit));
        }

        let total_count = commits.len();
        let response = LogResponse {
            repo_path: repo.path().parent().unwrap_or(Path::new(".")).to_string_lossy().to_string(),
            commits,
            total_count,
        };

        Ok(CallToolResult::success(vec![Content::json(&response)?]))
    }

    #[tool(description = "Get the diff between two references or working directory")]
    async fn git_diff(
        &self,
        Parameters(params): Parameters<DiffParams>,
    ) -> Result<CallToolResult, McpError> {
        let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;
        let include_patch = params.include_patch.unwrap_or(true);

        let mut diff_opts = DiffOptions::new();
        if let Some(path) = &params.path {
            diff_opts.pathspec(path);
        }

        let diff = if let Some(from_ref) = &params.from_ref {
            let from_obj = repo.revparse_single(from_ref).map_err(|_| git_error_to_mcp(GitError::RefNotFound(from_ref.clone())))?;
            let from_tree = from_obj.peel_to_tree().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

            if let Some(to_ref) = &params.to_ref {
                let to_obj = repo.revparse_single(to_ref).map_err(|_| git_error_to_mcp(GitError::RefNotFound(to_ref.clone())))?;
                let to_tree = to_obj.peel_to_tree().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
                repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), Some(&mut diff_opts))
            } else {
                repo.diff_tree_to_workdir_with_index(Some(&from_tree), Some(&mut diff_opts))
            }
        } else {
            // Default: diff HEAD to working directory
            let head = repo.head().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
            let head_tree = head.peel_to_tree().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
            repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut diff_opts))
        }
        .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

        let stats = diff.stats().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

        let diff_str = if include_patch {
            let mut diff_text = String::new();
            diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
                match line.origin() {
                    '+' | '-' | ' ' => {
                        diff_text.push(line.origin());
                    }
                    _ => {}
                }
                if let Ok(content) = std::str::from_utf8(line.content()) {
                    diff_text.push_str(content);
                }
                true
            })
            .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
            diff_text
        } else {
            String::new()
        };

        let response = DiffResponse {
            repo_path: repo.path().parent().unwrap_or(Path::new(".")).to_string_lossy().to_string(),
            from_ref: params.from_ref,
            to_ref: params.to_ref,
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
            diff: diff_str,
        };

        Ok(CallToolResult::success(vec![Content::json(&response)?]))
    }

    #[tool(description = "Show details of a specific commit")]
    async fn git_show(
        &self,
        Parameters(params): Parameters<ShowParams>,
    ) -> Result<CallToolResult, McpError> {
        let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;
        let include_diff = params.include_diff.unwrap_or(false);

        let obj = repo.revparse_single(&params.rev).map_err(|_| git_error_to_mcp(GitError::RefNotFound(params.rev.clone())))?;
        let commit = obj.peel_to_commit().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

        let tree = commit.tree().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        let diff = repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

        let stats = diff.stats().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

        let diff_str = if include_diff {
            let mut diff_text = String::new();
            diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
                match line.origin() {
                    '+' | '-' | ' ' => {
                        diff_text.push(line.origin());
                    }
                    _ => {}
                }
                if let Ok(content) = std::str::from_utf8(line.content()) {
                    diff_text.push_str(content);
                }
                true
            })
            .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
            Some(diff_text)
        } else {
            None
        };

        let response = ShowResponse {
            commit: commit_to_info(&commit),
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
            diff: diff_str,
        };

        Ok(CallToolResult::success(vec![Content::json(&response)?]))
    }

    #[tool(description = "List branches in a git repository")]
    async fn git_branch_list(
        &self,
        Parameters(params): Parameters<BranchListParams>,
    ) -> Result<CallToolResult, McpError> {
        let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;
        let include_remote = params.include_remote.unwrap_or(false);

        let head = repo.head().ok();
        let current_branch = head.as_ref().and_then(|h| {
            if h.is_branch() {
                h.shorthand().map(String::from)
            } else {
                None
            }
        });

        let mut branches = Vec::new();

        // Local branches
        for branch_result in repo.branches(Some(BranchType::Local)).map_err(|e| git_error_to_mcp(GitError::Git(e)))? {
            let (branch, _) = branch_result.map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
            let name = branch.name().map_err(|e| git_error_to_mcp(GitError::Git(e)))?.unwrap_or("").to_string();
            let is_head = branch.is_head();
            let commit_id = branch.get().target().map(|id| id.to_string());
            let upstream = branch.upstream().ok().and_then(|u| u.name().ok().flatten().map(String::from));

            branches.push(BranchInfo {
                name,
                is_head,
                commit_id,
                upstream,
                is_remote: false,
            });
        }

        // Remote branches
        if include_remote {
            for branch_result in repo.branches(Some(BranchType::Remote)).map_err(|e| git_error_to_mcp(GitError::Git(e)))? {
                let (branch, _) = branch_result.map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
                let name = branch.name().map_err(|e| git_error_to_mcp(GitError::Git(e)))?.unwrap_or("").to_string();
                let commit_id = branch.get().target().map(|id| id.to_string());

                branches.push(BranchInfo {
                    name,
                    is_head: false,
                    commit_id,
                    upstream: None,
                    is_remote: true,
                });
            }
        }

        let response = BranchListResponse {
            repo_path: repo.path().parent().unwrap_or(Path::new(".")).to_string_lossy().to_string(),
            current_branch,
            branches,
        };

        Ok(CallToolResult::success(vec![Content::json(&response)?]))
    }

    #[tool(description = "Show line-by-line authorship information (git blame) for a file")]
    async fn git_blame(
        &self,
        Parameters(params): Parameters<BlameParams>,
    ) -> Result<CallToolResult, McpError> {
        let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;

        let mut blame_opts = BlameOptions::new();
        if let Some(start) = params.start_line {
            blame_opts.min_line(start);
        }
        if let Some(end) = params.end_line {
            blame_opts.max_line(end);
        }

        let blame = repo
            .blame_file(Path::new(&params.file_path), Some(&mut blame_opts))
            .map_err(|e| {
                if e.code() == git2::ErrorCode::NotFound {
                    git_error_to_mcp(GitError::FileNotFound(params.file_path.clone()))
                } else {
                    git_error_to_mcp(GitError::Git(e))
                }
            })?;

        // Read file contents to get line content
        let workdir = repo.workdir().unwrap_or(Path::new("."));
        let file_path = workdir.join(&params.file_path);
        let file_content = std::fs::read_to_string(&file_path)
            .map_err(|_| git_error_to_mcp(GitError::FileNotFound(params.file_path.clone())))?;
        let file_lines: Vec<&str> = file_content.lines().collect();

        let mut lines = Vec::new();
        for hunk in blame.iter() {
            let sig = hunk.final_signature();
            let line_num = hunk.final_start_line();

            for offset in 0..hunk.lines_in_hunk() {
                let actual_line = line_num + offset;
                let content = file_lines.get(actual_line - 1).unwrap_or(&"").to_string();

                lines.push(BlameLine {
                    line_number: actual_line,
                    commit_id: hunk.final_commit_id().to_string(),
                    author: sig.name().unwrap_or("").to_string(),
                    author_time: git_time_to_datetime(sig.when()),
                    content,
                });
            }
        }

        let response = BlameResponse {
            repo_path: repo.path().parent().unwrap_or(Path::new(".")).to_string_lossy().to_string(),
            file_path: params.file_path,
            lines,
        };

        Ok(CallToolResult::success(vec![Content::json(&response)?]))
    }

    #[tool(description = "Manage git stashes: list, show, save, pop, apply, or drop")]
    async fn git_stash(
        &self,
        Parameters(params): Parameters<StashParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;
        let index = params.index.unwrap_or(0);

        let mut entries = Vec::new();
        let mut message = None;

        match params.action.as_str() {
            "list" => {
                repo.stash_foreach(|idx, msg, oid| {
                    entries.push(StashEntry {
                        index: idx,
                        message: msg.to_string(),
                        commit_id: oid.to_string(),
                    });
                    true
                })
                .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
            }
            "save" => {
                let sig = repo.signature().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
                let stash_msg = params.message.as_deref();
                let oid = repo
                    .stash_save(&sig, stash_msg.unwrap_or("WIP on stash"), None)
                    .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
                message = Some(format!("Saved working directory and index state: {}", oid));
            }
            "pop" => {
                repo.stash_pop(index, None)
                    .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
                message = Some(format!("Popped stash@{{{}}}", index));
            }
            "apply" => {
                repo.stash_apply(index, None)
                    .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
                message = Some(format!("Applied stash@{{{}}}", index));
            }
            "drop" => {
                repo.stash_drop(index)
                    .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
                message = Some(format!("Dropped stash@{{{}}}", index));
            }
            other => {
                return Err(McpError::invalid_params(
                    format!("Unknown stash action: {}. Use: list, save, pop, apply, drop", other),
                    None,
                ));
            }
        }

        let response = StashResponse {
            repo_path: repo.path().parent().unwrap_or(Path::new(".")).to_string_lossy().to_string(),
            action: params.action,
            entries,
            message,
        };

        Ok(CallToolResult::success(vec![Content::json(&response)?]))
    }

    #[tool(description = "List git remotes configured for the repository")]
    async fn git_remote_list(
        &self,
        Parameters(params): Parameters<RemoteListParams>,
    ) -> Result<CallToolResult, McpError> {
        let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;

        let remote_names = repo.remotes().map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

        let mut remotes = Vec::new();
        for name in remote_names.iter().flatten() {
            if let Ok(remote) = repo.find_remote(name) {
                remotes.push(RemoteInfo {
                    name: name.to_string(),
                    url: remote.url().map(String::from),
                    push_url: remote.pushurl().map(String::from),
                });
            }
        }

        let response = RemoteListResponse {
            repo_path: repo.path().parent().unwrap_or(Path::new(".")).to_string_lossy().to_string(),
            remotes,
        };

        Ok(CallToolResult::success(vec![Content::json(&response)?]))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for GitMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Git MCP server providing local git repository operations using libgit2. \
                 Complements GitHub API tools with local repository access."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
