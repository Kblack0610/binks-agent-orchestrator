//! Core git operation handlers: status, log, diff, show, branch, remote

use git2::{BranchType, DiffOptions, StatusOptions};
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use std::path::Path;

use crate::params::{
    BranchListParams, DiffParams, LogParams, RemoteListParams, ShowParams, StatusParams,
};
use crate::types::*;

use super::{commit_to_info, git_error_to_mcp, open_repo};

/// Get the status of a git repository
pub async fn status(params: StatusParams) -> Result<CallToolResult, McpError> {
    let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;

    let mut status_opts = StatusOptions::new();
    status_opts
        .include_untracked(true)
        .include_ignored(false)
        .include_unmodified(false);

    let statuses = repo
        .statuses(Some(&mut status_opts))
        .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

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
                    old_path: entry
                        .head_to_index()
                        .and_then(|d| d.old_file().path().map(|p| p.to_string_lossy().to_string())),
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
                    old_path: entry
                        .index_to_workdir()
                        .and_then(|d| d.old_file().path().map(|p| p.to_string_lossy().to_string())),
                });
            }
        }
    }

    let is_clean =
        staged.is_empty() && modified.is_empty() && untracked.is_empty() && conflicted.is_empty();

    let response = StatusResponse {
        repo_path: repo
            .path()
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string(),
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

/// Get the commit log of a git repository
pub async fn log(params: LogParams) -> Result<CallToolResult, McpError> {
    let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;
    let limit = params.limit.unwrap_or(10);

    let mut revwalk = repo
        .revwalk()
        .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

    // Set starting point
    if let Some(rev) = &params.rev {
        let obj = repo
            .revparse_single(rev)
            .map_err(|_| git_error_to_mcp(GitError::RefNotFound(rev.clone())))?;
        revwalk
            .push(obj.id())
            .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
    } else {
        revwalk
            .push_head()
            .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
    }

    revwalk
        .set_sorting(git2::Sort::TIME)
        .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

    let mut commits = Vec::new();
    for (count, oid_result) in revwalk.enumerate() {
        if count >= limit {
            break;
        }

        let oid = oid_result.map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
        let commit = repo
            .find_commit(oid)
            .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

        // Path filter if specified
        if let Some(filter_path) = &params.path {
            let tree = commit
                .tree()
                .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
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
        repo_path: repo
            .path()
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string(),
        commits,
        total_count,
    };

    Ok(CallToolResult::success(vec![Content::json(&response)?]))
}

/// Get the diff between two references or working directory
pub async fn diff(params: DiffParams) -> Result<CallToolResult, McpError> {
    let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;
    let include_patch = params.include_patch.unwrap_or(true);

    let mut diff_opts = DiffOptions::new();
    if let Some(path) = &params.path {
        diff_opts.pathspec(path);
    }

    let diff = if let Some(from_ref) = &params.from_ref {
        let from_obj = repo
            .revparse_single(from_ref)
            .map_err(|_| git_error_to_mcp(GitError::RefNotFound(from_ref.clone())))?;
        let from_tree = from_obj
            .peel_to_tree()
            .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

        if let Some(to_ref) = &params.to_ref {
            let to_obj = repo
                .revparse_single(to_ref)
                .map_err(|_| git_error_to_mcp(GitError::RefNotFound(to_ref.clone())))?;
            let to_tree = to_obj
                .peel_to_tree()
                .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
            repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), Some(&mut diff_opts))
        } else {
            repo.diff_tree_to_workdir_with_index(Some(&from_tree), Some(&mut diff_opts))
        }
    } else {
        // Default: diff HEAD to working directory
        let head = repo
            .head()
            .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
        let head_tree = head
            .peel_to_tree()
            .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
        repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut diff_opts))
    }
    .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

    let stats = diff
        .stats()
        .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

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
        repo_path: repo
            .path()
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string(),
        from_ref: params.from_ref,
        to_ref: params.to_ref,
        files_changed: stats.files_changed(),
        insertions: stats.insertions(),
        deletions: stats.deletions(),
        diff: diff_str,
    };

    Ok(CallToolResult::success(vec![Content::json(&response)?]))
}

/// Show details of a specific commit
pub async fn show(params: ShowParams) -> Result<CallToolResult, McpError> {
    let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;
    let include_diff = params.include_diff.unwrap_or(false);

    let obj = repo
        .revparse_single(&params.rev)
        .map_err(|_| git_error_to_mcp(GitError::RefNotFound(params.rev.clone())))?;
    let commit = obj
        .peel_to_commit()
        .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

    let tree = commit
        .tree()
        .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
        .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

    let stats = diff
        .stats()
        .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

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

/// List branches in a git repository
pub async fn branch_list(params: BranchListParams) -> Result<CallToolResult, McpError> {
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
    for branch_result in repo
        .branches(Some(BranchType::Local))
        .map_err(|e| git_error_to_mcp(GitError::Git(e)))?
    {
        let (branch, _) = branch_result.map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
        let name = branch
            .name()
            .map_err(|e| git_error_to_mcp(GitError::Git(e)))?
            .unwrap_or("")
            .to_string();
        let is_head = branch.is_head();
        let commit_id = branch.get().target().map(|id| id.to_string());
        let upstream = branch
            .upstream()
            .ok()
            .and_then(|u| u.name().ok().flatten().map(String::from));

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
        for branch_result in repo
            .branches(Some(BranchType::Remote))
            .map_err(|e| git_error_to_mcp(GitError::Git(e)))?
        {
            let (branch, _) = branch_result.map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
            let name = branch
                .name()
                .map_err(|e| git_error_to_mcp(GitError::Git(e)))?
                .unwrap_or("")
                .to_string();
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
        repo_path: repo
            .path()
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string(),
        current_branch,
        branches,
    };

    Ok(CallToolResult::success(vec![Content::json(&response)?]))
}

/// List git remotes configured for the repository
pub async fn remote_list(params: RemoteListParams) -> Result<CallToolResult, McpError> {
    let repo = open_repo(params.repo_path.as_deref()).map_err(git_error_to_mcp)?;

    let remote_names = repo
        .remotes()
        .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;

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
        repo_path: repo
            .path()
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string(),
        remotes,
    };

    Ok(CallToolResult::success(vec![Content::json(&response)?]))
}
