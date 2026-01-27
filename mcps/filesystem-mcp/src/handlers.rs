//! Filesystem operation handlers
//!
//! Each handler takes the sandbox, config, and params to perform file operations.

use chrono::{DateTime, Utc};
use mcp_common::{internal_error, invalid_params, json_success, CallToolResult, McpError};
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::params::*;
use crate::sandbox::Sandbox;
use crate::types::{
    Config, DeleteFileResponse, DirectoryTreeResponse, EditFileResponse, FileEntry,
    FileInfoResponse, FileReadResult, FsError, ListDirResponse, MoveFileResponse,
    ReadFileResponse, ReadMultipleFilesResponse, SearchFilesResponse, TreeEntry,
    WriteFileResponse,
};

// ============================================================================
// Helper Functions
// ============================================================================

pub fn fs_error_to_mcp(err: FsError) -> McpError {
    match &err {
        FsError::AccessDenied(_) | FsError::PathTraversal(_) => {
            McpError::invalid_request(err.to_string(), None)
        }
        FsError::NotFound(_) => invalid_params(err.to_string()),
        FsError::FileTooLarge { .. } => McpError::invalid_request(err.to_string(), None),
        _ => internal_error(err.to_string()),
    }
}

async fn get_file_entry(path: &Path) -> Result<FileEntry, std::io::Error> {
    let metadata = fs::metadata(path).await?;
    let modified: Option<DateTime<Utc>> = metadata.modified().ok().map(|t| t.into());

    Ok(FileEntry {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        path: path.display().to_string(),
        entry_type: if metadata.is_dir() {
            "directory".to_string()
        } else {
            "file".to_string()
        },
        size: if metadata.is_file() {
            Some(metadata.len())
        } else {
            None
        },
        modified,
    })
}

// ============================================================================
// Handler Functions
// ============================================================================

pub async fn read_file(
    sandbox: &Sandbox,
    config: &Config,
    params: ReadFileParams,
) -> Result<CallToolResult, McpError> {
    let canonical = sandbox
        .validate_read(&params.path)
        .map_err(fs_error_to_mcp)?;

    // Check file size before reading
    let metadata = fs::metadata(&canonical)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    if metadata.len() > config.limits.max_file_size as u64 {
        return Err(fs_error_to_mcp(FsError::FileTooLarge {
            size: metadata.len(),
            max: config.limits.max_file_size,
        }));
    }

    let content = fs::read_to_string(&canonical)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let response = ReadFileResponse {
        path: canonical.display().to_string(),
        content,
        size: metadata.len(),
    };

    json_success(&response)
}

pub async fn write_file(
    sandbox: &Sandbox,
    config: &Config,
    params: WriteFileParams,
) -> Result<CallToolResult, McpError> {
    let canonical = sandbox
        .validate_write(&params.path)
        .map_err(fs_error_to_mcp)?;

    // Check content size
    if params.content.len() > config.limits.max_file_size {
        return Err(fs_error_to_mcp(FsError::FileTooLarge {
            size: params.content.len() as u64,
            max: config.limits.max_file_size,
        }));
    }

    // Atomic write: write to temp file then rename to target
    let parent = canonical
        .parent()
        .ok_or_else(|| internal_error("Cannot determine parent directory".to_string()))?;
    let file_name = canonical
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let temp_path = parent.join(format!(
        ".{}.tmp.{}",
        file_name,
        std::process::id()
    ));

    // Write to temp file
    let write_result = async {
        let mut file = fs::File::create(&temp_path)
            .await
            .map_err(|e| internal_error(format!("Failed to create temp file: {}", e)))?;
        file.write_all(params.content.as_bytes())
            .await
            .map_err(|e| internal_error(format!("Failed to write temp file: {}", e)))?;
        file.sync_all()
            .await
            .map_err(|e| internal_error(format!("Failed to sync temp file: {}", e)))?;
        Ok::<(), McpError>(())
    }
    .await;

    // If write failed, clean up temp file
    if let Err(e) = write_result {
        let _ = fs::remove_file(&temp_path).await;
        return Err(e);
    }

    // Atomic rename
    if let Err(e) = fs::rename(&temp_path, &canonical).await {
        let _ = fs::remove_file(&temp_path).await;
        return Err(internal_error(format!("Failed to rename temp file: {}", e)));
    }

    let response = WriteFileResponse {
        path: canonical.display().to_string(),
        success: true,
        bytes_written: params.content.len(),
    };

    json_success(&response)
}

pub async fn edit_file(
    sandbox: &Sandbox,
    config: &Config,
    params: EditFileParams,
) -> Result<CallToolResult, McpError> {
    let canonical = sandbox
        .validate_write(&params.path)
        .map_err(fs_error_to_mcp)?;

    // Read existing content
    let content = fs::read_to_string(&canonical)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let new_content = if params.old_string.is_empty() {
        // Empty old_string → prepend new_string to file
        format!("{}{}", params.new_string, content)
    } else {
        // Count occurrences
        let count = content.matches(&params.old_string).count();
        if count == 0 {
            return Err(invalid_params(format!(
                "old_string not found in {}",
                canonical.display()
            )));
        }
        if count > 1 {
            return Err(invalid_params(format!(
                "old_string found {} times in {} — provide more context to make the match unique",
                count,
                canonical.display()
            )));
        }
        content.replacen(&params.old_string, &params.new_string, 1)
    };

    // Check size limit
    if new_content.len() > config.limits.max_file_size {
        return Err(fs_error_to_mcp(FsError::FileTooLarge {
            size: new_content.len() as u64,
            max: config.limits.max_file_size,
        }));
    }

    // Write back
    fs::write(&canonical, &new_content)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    // Build a snippet around the edit location
    let snippet = build_edit_snippet(&new_content, &params.new_string);

    let response = EditFileResponse {
        path: canonical.display().to_string(),
        success: true,
        new_size: new_content.len() as u64,
        snippet,
    };

    json_success(&response)
}

/// Build a context snippet showing lines around the edited region
fn build_edit_snippet(content: &str, new_text: &str) -> String {
    const CONTEXT_LINES: usize = 3;

    if new_text.is_empty() {
        return "(text deleted)".to_string();
    }

    // Find the position of the replacement text
    let Some(byte_offset) = content.find(new_text) else {
        return String::new();
    };

    let lines: Vec<&str> = content.lines().collect();

    // Find which line the edit starts on
    let mut edit_line = 0;
    let mut chars_seen = 0;
    for (i, line) in lines.iter().enumerate() {
        if chars_seen + line.len() >= byte_offset {
            edit_line = i;
            break;
        }
        chars_seen += line.len() + 1; // +1 for newline
    }

    // Find how many lines the replacement spans
    let replacement_lines = new_text.lines().count().max(1);
    let edit_end_line = (edit_line + replacement_lines).min(lines.len());

    let start = edit_line.saturating_sub(CONTEXT_LINES);
    let end = (edit_end_line + CONTEXT_LINES).min(lines.len());

    lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:>4} | {}", start + i + 1, line))
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn list_dir(
    sandbox: &Sandbox,
    config: &Config,
    params: ListDirParams,
) -> Result<CallToolResult, McpError> {
    let canonical = sandbox
        .validate_read(&params.path)
        .map_err(fs_error_to_mcp)?;

    let mut entries = Vec::new();
    let mut count = 0;

    if params.recursive {
        // Recursive listing
        let mut stack = vec![canonical.clone()];
        while let Some(dir) = stack.pop() {
            if count >= config.limits.max_files_per_list {
                break;
            }

            let mut read_dir = fs::read_dir(&dir)
                .await
                .map_err(|e| internal_error(e.to_string()))?;

            while let Some(entry) = read_dir
                .next_entry()
                .await
                .map_err(|e| internal_error(e.to_string()))?
            {
                if count >= config.limits.max_files_per_list {
                    break;
                }

                let path = entry.path();

                // Verify path is still allowed
                if sandbox.check_read(&path).is_err() {
                    continue;
                }

                if let Ok(file_entry) = get_file_entry(&path).await {
                    if file_entry.entry_type == "directory" {
                        stack.push(path);
                    }
                    entries.push(file_entry);
                    count += 1;
                }
            }
        }
    } else {
        // Non-recursive listing
        let mut read_dir = fs::read_dir(&canonical)
            .await
            .map_err(|e| internal_error(e.to_string()))?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| internal_error(e.to_string()))?
        {
            if count >= config.limits.max_files_per_list {
                break;
            }

            let path = entry.path();
            if let Ok(file_entry) = get_file_entry(&path).await {
                entries.push(file_entry);
                count += 1;
            }
        }
    }

    let response = ListDirResponse {
        path: canonical.display().to_string(),
        entries,
        total_count: count,
    };

    json_success(&response)
}

pub async fn search_files(
    sandbox: &Sandbox,
    config: &Config,
    params: SearchFilesParams,
) -> Result<CallToolResult, McpError> {
    let canonical = sandbox
        .validate_read(&params.path)
        .map_err(fs_error_to_mcp)?;

    // Combine base path with pattern
    let full_pattern = canonical.join(&params.pattern);
    let pattern_str = full_pattern.display().to_string();

    let mut matches = Vec::new();
    let mut count = 0;

    // Use glob to find matches
    for entry in glob::glob(&pattern_str).map_err(|e| invalid_params(e.to_string()))? {
        if count >= config.limits.max_files_per_list {
            break;
        }

        if let Ok(path) = entry {
            // Verify path is allowed
            if sandbox.check_read(&path).is_ok() {
                matches.push(path.display().to_string());
                count += 1;
            }
        }
    }

    let response = SearchFilesResponse {
        pattern: params.pattern,
        base_path: canonical.display().to_string(),
        matches,
        total_count: count,
    };

    json_success(&response)
}

pub async fn file_info(
    sandbox: &Sandbox,
    params: FileInfoParams,
) -> Result<CallToolResult, McpError> {
    let canonical = sandbox
        .validate_read(&params.path)
        .map_err(fs_error_to_mcp)?;

    let response = if canonical.exists() {
        let metadata = fs::metadata(&canonical)
            .await
            .map_err(|e| internal_error(e.to_string()))?;

        let modified: Option<DateTime<Utc>> = metadata.modified().ok().map(|t| t.into());
        let created: Option<DateTime<Utc>> = metadata.created().ok().map(|t| t.into());

        FileInfoResponse {
            path: canonical.display().to_string(),
            exists: true,
            entry_type: Some(if metadata.is_dir() {
                "directory".to_string()
            } else if metadata.is_symlink() {
                "symlink".to_string()
            } else {
                "file".to_string()
            }),
            size: if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            },
            modified,
            created,
            readonly: Some(metadata.permissions().readonly()),
        }
    } else {
        FileInfoResponse {
            path: canonical.display().to_string(),
            exists: false,
            entry_type: None,
            size: None,
            modified: None,
            created: None,
            readonly: None,
        }
    };

    json_success(&response)
}

pub async fn move_file(
    sandbox: &Sandbox,
    params: MoveFileParams,
) -> Result<CallToolResult, McpError> {
    // Source must be readable, destination must be writable
    let src_canonical = sandbox
        .validate_read(&params.src)
        .map_err(fs_error_to_mcp)?;
    let dst_canonical = sandbox
        .validate_write(&params.dst)
        .map_err(fs_error_to_mcp)?;

    fs::rename(&src_canonical, &dst_canonical)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let response = MoveFileResponse {
        src: src_canonical.display().to_string(),
        dst: dst_canonical.display().to_string(),
        success: true,
    };

    json_success(&response)
}

pub async fn delete_file(
    sandbox: &Sandbox,
    params: DeleteFileParams,
) -> Result<CallToolResult, McpError> {
    let canonical = sandbox
        .validate_write(&params.path)
        .map_err(fs_error_to_mcp)?;

    if !canonical.exists() {
        return Err(fs_error_to_mcp(FsError::NotFound(
            canonical.display().to_string(),
        )));
    }

    let metadata = fs::metadata(&canonical)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    if metadata.is_dir() {
        if params.recursive {
            fs::remove_dir_all(&canonical)
                .await
                .map_err(|e| internal_error(e.to_string()))?;
        } else {
            fs::remove_dir(&canonical)
                .await
                .map_err(|e| internal_error(e.to_string()))?;
        }
    } else {
        fs::remove_file(&canonical)
            .await
            .map_err(|e| internal_error(e.to_string()))?;
    }

    let response = DeleteFileResponse {
        path: canonical.display().to_string(),
        deleted: true,
    };

    json_success(&response)
}

pub async fn create_directory(
    sandbox: &Sandbox,
    params: CreateDirParams,
) -> Result<CallToolResult, McpError> {
    let canonical = sandbox
        .validate_write(&params.path)
        .map_err(fs_error_to_mcp)?;

    if params.recursive {
        fs::create_dir_all(&canonical)
            .await
            .map_err(|e| internal_error(e.to_string()))?;
    } else {
        fs::create_dir(&canonical)
            .await
            .map_err(|e| internal_error(e.to_string()))?;
    }

    let response = serde_json::json!({
        "path": canonical.display().to_string(),
        "created": true
    });

    json_success(&response)
}

pub async fn list_allowed_directories(sandbox: &Sandbox) -> Result<CallToolResult, McpError> {
    let response = serde_json::json!({
        "read_paths": sandbox.allowed_read_paths(),
        "write_paths": sandbox.allowed_write_paths()
    });

    json_success(&response)
}

pub async fn read_multiple_files(
    sandbox: &Sandbox,
    config: &Config,
    params: ReadMultipleFilesParams,
) -> Result<CallToolResult, McpError> {
    let mut results = Vec::with_capacity(params.paths.len());
    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for path_str in &params.paths {
        match read_single_file(sandbox, config, path_str).await {
            Ok((content, size)) => {
                results.push(FileReadResult {
                    path: path_str.clone(),
                    content: Some(content),
                    size: Some(size),
                    error: None,
                });
                succeeded += 1;
            }
            Err(e) => {
                results.push(FileReadResult {
                    path: path_str.clone(),
                    content: None,
                    size: None,
                    error: Some(e),
                });
                failed += 1;
            }
        }
    }

    let total = results.len();
    let response = ReadMultipleFilesResponse {
        results,
        total,
        succeeded,
        failed,
    };

    json_success(&response)
}

/// Read a single file, returning (content, size) or an error string
async fn read_single_file(
    sandbox: &Sandbox,
    config: &Config,
    path: &str,
) -> Result<(String, u64), String> {
    let canonical = sandbox.validate_read(path).map_err(|e| e.to_string())?;

    let metadata = fs::metadata(&canonical)
        .await
        .map_err(|e| e.to_string())?;

    if metadata.len() > config.limits.max_file_size as u64 {
        return Err(format!(
            "File too large: {} bytes (max {})",
            metadata.len(),
            config.limits.max_file_size
        ));
    }

    let content = fs::read_to_string(&canonical)
        .await
        .map_err(|e| e.to_string())?;

    Ok((content, metadata.len()))
}

pub async fn directory_tree(
    sandbox: &Sandbox,
    config: &Config,
    params: DirectoryTreeParams,
) -> Result<CallToolResult, McpError> {
    let canonical = sandbox
        .validate_read(&params.path)
        .map_err(fs_error_to_mcp)?;

    let max_depth = params.depth.unwrap_or(3).min(10) as usize;
    let mut total_entries = 0usize;

    let tree = build_tree_entry(
        sandbox,
        &canonical,
        0,
        max_depth,
        config.limits.max_files_per_list,
        &mut total_entries,
    )
    .await
    .map_err(|e| internal_error(e.to_string()))?;

    let response = DirectoryTreeResponse {
        tree,
        total_entries,
    };

    json_success(&response)
}

/// Recursively build a tree of directory entries
async fn build_tree_entry(
    sandbox: &Sandbox,
    path: &Path,
    current_depth: usize,
    max_depth: usize,
    max_entries: usize,
    total_entries: &mut usize,
) -> Result<TreeEntry, std::io::Error> {
    let metadata = fs::metadata(path).await?;
    let is_dir = metadata.is_dir();

    let mut entry = TreeEntry {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string()),
        path: path.display().to_string(),
        entry_type: if is_dir {
            "directory".to_string()
        } else {
            "file".to_string()
        },
        size: if is_dir { None } else { Some(metadata.len()) },
        children: None,
    };

    *total_entries += 1;

    if is_dir && current_depth < max_depth && *total_entries < max_entries {
        let mut children = Vec::new();
        let mut read_dir = fs::read_dir(path).await?;

        while let Some(child) = read_dir.next_entry().await? {
            if *total_entries >= max_entries {
                break;
            }

            let child_path = child.path();

            // Skip paths not allowed by sandbox
            if sandbox.check_read(&child_path).is_err() {
                continue;
            }

            match Box::pin(build_tree_entry(
                sandbox,
                &child_path,
                current_depth + 1,
                max_depth,
                max_entries,
                total_entries,
            ))
            .await
            {
                Ok(child_entry) => children.push(child_entry),
                Err(_) => continue, // Skip unreadable entries
            }
        }

        entry.children = Some(children);
    }

    Ok(entry)
}
