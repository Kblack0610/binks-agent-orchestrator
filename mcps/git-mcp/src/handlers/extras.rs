//! Extra git operation handlers: blame and stash

use git2::BlameOptions;
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use std::path::Path;

use crate::params::{BlameParams, StashParams};
use crate::types::*;

use super::{git_error_to_mcp, git_time_to_datetime, open_repo};

/// Show line-by-line authorship information (git blame) for a file
pub async fn blame(params: BlameParams) -> Result<CallToolResult, McpError> {
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
        repo_path: repo
            .path()
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string(),
        file_path: params.file_path,
        lines,
    };

    Ok(CallToolResult::success(vec![Content::json(&response)?]))
}

/// Manage git stashes: list, show, save, pop, apply, or drop
pub async fn stash(params: StashParams) -> Result<CallToolResult, McpError> {
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
            let sig = repo
                .signature()
                .map_err(|e| git_error_to_mcp(GitError::Git(e)))?;
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
                format!(
                    "Unknown stash action: {}. Use: list, save, pop, apply, drop",
                    other
                ),
                None,
            ));
        }
    }

    let response = StashResponse {
        repo_path: repo
            .path()
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string(),
        action: params.action,
        entries,
        message,
    };

    Ok(CallToolResult::success(vec![Content::json(&response)?]))
}
