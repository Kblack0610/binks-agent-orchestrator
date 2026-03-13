//! Ingestion pipeline: file discovery, markdown chunking, hash-based skip
//!
//! Handles glob expansion, exclude filtering, dedup, and chunk splitting.

use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::config::{self, KnowledgeConfig, SourceConfig};
use crate::docs_store::DocStore;
use crate::types::{KnowledgeError, SyncResponse};

/// Run the ingestion pipeline for configured sources.
///
/// Filters by repo, source_name, or path_prefix if provided.
pub async fn run_sync(
    store: &DocStore,
    config: &KnowledgeConfig,
    repo_filter: Option<&str>,
    source_filter: Option<&str>,
    path_prefix: Option<&str>,
    force: bool,
) -> Result<SyncResponse, KnowledgeError> {
    let start = Instant::now();
    let mut response = SyncResponse {
        sources_synced: 0,
        documents_added: 0,
        documents_updated: 0,
        documents_unchanged: 0,
        documents_skipped: 0,
        documents_removed: 0,
        duration_ms: 0,
    };

    for source in &config.sources {
        if !source.enabled {
            continue;
        }
        if let Some(rf) = repo_filter {
            if source.repo != rf {
                continue;
            }
        }
        if let Some(sf) = source_filter {
            if source.name != sf {
                continue;
            }
        }

        match sync_source(store, config, source, path_prefix, force).await {
            Ok(stats) => {
                response.sources_synced += 1;
                response.documents_added += stats.added;
                response.documents_updated += stats.updated;
                response.documents_unchanged += stats.unchanged;
                response.documents_skipped += stats.skipped;
                response.documents_removed += stats.removed;
            }
            Err(e) => {
                tracing::warn!(source = %source.name, error = %e, "Failed to sync source");
            }
        }
    }

    response.duration_ms = start.elapsed().as_millis() as u64;
    Ok(response)
}

struct SourceStats {
    added: usize,
    updated: usize,
    unchanged: usize,
    skipped: usize,
    removed: usize,
}

async fn sync_source(
    store: &DocStore,
    config: &KnowledgeConfig,
    source: &SourceConfig,
    path_prefix: Option<&str>,
    force: bool,
) -> Result<SourceStats, KnowledgeError> {
    let base_path = PathBuf::from(&source.base_path);

    if !base_path.exists() {
        tracing::warn!(source = %source.name, path = %base_path.display(), "Source base_path does not exist, skipping");
        return Ok(SourceStats {
            added: 0,
            updated: 0,
            unchanged: 0,
            skipped: 0,
            removed: 0,
        });
    }

    // Upsert the source record
    let source_id = store
        .upsert_source(
            &source.name,
            &source.repo,
            &source.base_path,
            "docs",
            source.enabled,
        )
        .await?;

    // Resolve git HEAD once per source
    let commit_hash = resolve_git_head(&base_path);

    // Discover files via glob patterns
    let files = discover_files(&base_path, &source.patterns, &source.exclude_patterns)?;

    // Deduplicate by absolute path
    let mut seen_paths = HashSet::new();
    let deduped: Vec<PathBuf> = files
        .into_iter()
        .filter(|p| seen_paths.insert(p.clone()))
        .collect();

    // Track current file paths for stale removal
    let mut current_file_paths = Vec::new();

    let mut stats = SourceStats {
        added: 0,
        updated: 0,
        unchanged: 0,
        skipped: 0,
        removed: 0,
    };

    for file_path in &deduped {
        let abs_path = file_path.to_string_lossy().to_string();

        // Compute relative path from base
        let rel_path = file_path
            .strip_prefix(&base_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        // Apply path_prefix filter if present
        if let Some(prefix) = path_prefix {
            if !rel_path.starts_with(prefix) {
                continue;
            }
        }

        // Check file size
        let metadata = match std::fs::metadata(file_path) {
            Ok(m) => m,
            Err(_) => {
                stats.skipped += 1;
                continue;
            }
        };

        if metadata.len() > config.defaults.max_file_size {
            tracing::debug!(path = %rel_path, size = metadata.len(), "Skipping file: too large");
            stats.skipped += 1;
            continue;
        }

        // Read file content
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(path = %rel_path, error = %e, "Skipping non-UTF8 or unreadable file");
                stats.skipped += 1;
                continue;
            }
        };

        // Compute content hash
        let content_hash = sha256_hash(&content);

        // Check if unchanged (skip unless force)
        if !force {
            if let Ok(Some(existing_hash)) = store.get_doc_hash(&abs_path).await {
                if existing_hash == content_hash {
                    stats.unchanged += 1;
                    current_file_paths.push(abs_path);
                    continue;
                }
            }
        }

        // Resolve kind and priority
        let kind = config::resolve_kind(source, &rel_path);
        let priority = config::resolve_priority(source, &rel_path);

        // Extract title from first markdown heading
        let title = extract_title(&content);

        // Get file mtime
        let file_mtime = metadata
            .modified()
            .ok()
            .map(|t| {
                chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()
            });

        // Chunk the content
        let raw_chunks = chunk_markdown(&content);

        // Convert to the format expected by upsert_document
        let chunk_tuples: Vec<(Option<&str>, &str, i64, i64)> = raw_chunks
            .iter()
            .map(|c| {
                (
                    c.heading.as_deref(),
                    c.content.as_str(),
                    c.byte_offset as i64,
                    c.byte_length as i64,
                )
            })
            .collect();

        let was_update = store
            .upsert_document(
                &source_id,
                &source.repo,
                &abs_path,
                &rel_path,
                "docs",
                &kind,
                priority,
                title.as_deref(),
                &content,
                &content_hash,
                file_mtime.as_deref(),
                commit_hash.as_deref(),
                &chunk_tuples,
            )
            .await?;

        if was_update {
            stats.updated += 1;
        } else {
            stats.added += 1;
        }

        current_file_paths.push(abs_path);
    }

    // Remove docs that no longer exist in this source
    let removed = store
        .remove_stale_docs(&source_id, &current_file_paths)
        .await?;
    if removed > 0 {
        tracing::info!(source = %source.name, removed, "Removed deleted documents");
    }
    stats.removed = removed;

    Ok(stats)
}

/// Discover files matching glob patterns, excluding exclude_patterns
fn discover_files(
    base_path: &Path,
    patterns: &[String],
    exclude_patterns: &[String],
) -> Result<Vec<PathBuf>, KnowledgeError> {
    let mut files = Vec::new();

    for pattern in patterns {
        let full_pattern = base_path.join(pattern).to_string_lossy().to_string();
        match glob::glob(&full_pattern) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if entry.is_file() {
                        // Check exclude patterns against relative path
                        let rel = entry
                            .strip_prefix(base_path)
                            .unwrap_or(&entry)
                            .to_string_lossy()
                            .to_string();

                        let excluded = exclude_patterns
                            .iter()
                            .any(|ep| config::path_matches_glob(ep, &rel));

                        if !excluded {
                            files.push(entry);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(pattern, error = %e, "Invalid glob pattern");
            }
        }
    }

    Ok(files)
}

/// Resolve git HEAD for a repository
fn resolve_git_head(base_path: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["-C", &base_path.to_string_lossy(), "rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}

/// Compute SHA-256 hash of content
fn sha256_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Extract title from the first markdown heading
fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix('#') {
            // Strip leading # characters and whitespace
            let title = rest.trim_start_matches('#').trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

/// A raw chunk from markdown splitting
struct RawChunk {
    heading: Option<String>,
    content: String,
    byte_offset: usize,
    byte_length: usize,
}

/// Split markdown content by heading boundaries.
///
/// - Chunk 0 = preamble (content before first heading)
/// - Each heading starts a new chunk
/// - Soft limit ~4000 chars per chunk; sub-split on blank lines if exceeded
fn chunk_markdown(content: &str) -> Vec<RawChunk> {
    let mut chunks = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_content = String::new();
    let mut current_offset: usize = 0;
    let mut chunk_start_offset: usize = 0;

    for line in content.lines() {
        let line_len = line.len() + 1; // +1 for newline

        if line.starts_with('#') && line.contains(' ') {
            // Save previous chunk if it has content
            if !current_content.is_empty() {
                let trimmed = current_content.trim_end().to_string();
                let byte_length = trimmed.len();
                maybe_subsplit(
                    &mut chunks,
                    current_heading.take(),
                    trimmed,
                    chunk_start_offset,
                    byte_length,
                );
            }

            // Start new chunk
            current_heading = Some(line.trim_start_matches('#').trim().to_string());
            current_content = format!("{line}\n");
            chunk_start_offset = current_offset;
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }

        current_offset += line_len;
    }

    // Don't forget the last chunk
    if !current_content.is_empty() {
        let trimmed = current_content.trim_end().to_string();
        let byte_length = trimmed.len();
        maybe_subsplit(
            &mut chunks,
            current_heading,
            trimmed,
            chunk_start_offset,
            byte_length,
        );
    }

    // If content was empty, add a single empty chunk
    if chunks.is_empty() {
        chunks.push(RawChunk {
            heading: None,
            content: String::new(),
            byte_offset: 0,
            byte_length: 0,
        });
    }

    chunks
}

/// If content exceeds soft limit, sub-split on blank lines
const CHUNK_SOFT_LIMIT: usize = 4000;

fn maybe_subsplit(
    chunks: &mut Vec<RawChunk>,
    heading: Option<String>,
    content: String,
    byte_offset: usize,
    _byte_length: usize,
) {
    if content.len() <= CHUNK_SOFT_LIMIT {
        let len = content.len();
        chunks.push(RawChunk {
            heading,
            content,
            byte_offset,
            byte_length: len,
        });
        return;
    }

    // Sub-split on blank lines
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut running_offset = byte_offset;

    for line in content.lines() {
        if line.trim().is_empty() && current.len() >= CHUNK_SOFT_LIMIT {
            // Split here
            let len = current.len();
            parts.push((current.clone(), running_offset, len));
            running_offset += len + 1; // +1 for the blank line
            current.clear();
        } else {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }

    if !current.is_empty() {
        let len = current.len();
        parts.push((current, running_offset, len));
    }

    if parts.is_empty() {
        // Content couldn't be split, keep as-is
        let len = content.len();
        chunks.push(RawChunk {
            heading,
            content,
            byte_offset,
            byte_length: len,
        });
    } else {
        // First part gets the heading, rest get None
        for (i, (text, off, len)) in parts.into_iter().enumerate() {
            chunks.push(RawChunk {
                heading: if i == 0 { heading.clone() } else { None },
                content: text,
                byte_offset: off,
                byte_length: len,
            });
        }
    }
}
