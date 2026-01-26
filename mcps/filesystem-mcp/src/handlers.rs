//! Filesystem operation handlers
//!
//! Each handler takes the sandbox, config, and params to perform file operations.

use chrono::{DateTime, Utc};
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::params::*;
use crate::sandbox::Sandbox;
use crate::types::{
    Config, DeleteFileResponse, FileEntry, FileInfoResponse, FsError, ListDirResponse,
    MoveFileResponse, ReadFileResponse, SearchFilesResponse, WriteFileResponse,
};

// ============================================================================
// Helper Functions
// ============================================================================

pub fn fs_error_to_mcp(err: FsError) -> McpError {
    match &err {
        FsError::AccessDenied(_) | FsError::PathTraversal(_) => {
            McpError::invalid_request(err.to_string(), None)
        }
        FsError::NotFound(_) => McpError::invalid_params(err.to_string(), None),
        FsError::FileTooLarge { .. } => McpError::invalid_request(err.to_string(), None),
        _ => McpError::internal_error(err.to_string(), None),
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
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    if metadata.len() > config.limits.max_file_size as u64 {
        return Err(fs_error_to_mcp(FsError::FileTooLarge {
            size: metadata.len(),
            max: config.limits.max_file_size,
        }));
    }

    let content = fs::read_to_string(&canonical)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let response = ReadFileResponse {
        path: canonical.display().to_string(),
        content,
        size: metadata.len(),
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
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

    let mut file = fs::File::create(&canonical)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    file.write_all(params.content.as_bytes())
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let response = WriteFileResponse {
        path: canonical.display().to_string(),
        success: true,
        bytes_written: params.content.len(),
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
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
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            while let Some(entry) = read_dir
                .next_entry()
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
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

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
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
    for entry in
        glob::glob(&pattern_str).map_err(|e| McpError::invalid_params(e.to_string(), None))?
    {
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

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

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

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
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
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let response = MoveFileResponse {
        src: src_canonical.display().to_string(),
        dst: dst_canonical.display().to_string(),
        success: true,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
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
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    if metadata.is_dir() {
        if params.recursive {
            fs::remove_dir_all(&canonical)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        } else {
            fs::remove_dir(&canonical)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }
    } else {
        fs::remove_file(&canonical)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    }

    let response = DeleteFileResponse {
        path: canonical.display().to_string(),
        deleted: true,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    } else {
        fs::create_dir(&canonical)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    }

    let response = serde_json::json!({
        "path": canonical.display().to_string(),
        "created": true
    });

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

pub async fn list_allowed_directories(sandbox: &Sandbox) -> Result<CallToolResult, McpError> {
    let response = serde_json::json!({
        "read_paths": sandbox.allowed_read_paths(),
        "write_paths": sandbox.allowed_write_paths()
    });

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}
