//! MCP tool handlers for knowledge-mcp
//!
//! Each handler takes the store + config and params to perform operations.

use mcp_common::{internal_error, json_success, CallToolResult, McpError};

use crate::config::KnowledgeConfig;
use crate::docs_store::DocStore;
use crate::ingest;
use crate::params::*;
use crate::project_notes;
use crate::types::*;

pub async fn search_docs(
    store: &DocStore,
    params: SearchDocsParams,
) -> Result<CallToolResult, McpError> {
    let (results, total_matches) = store
        .search(
            &params.query,
            params.repo.as_deref(),
            params.kind.as_deref(),
            params.limit,
        )
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let response = SearchDocsResponse {
        results,
        total_matches,
        query: params.query,
    };

    json_success(&response)
}

pub async fn get_doc(store: &DocStore, params: GetDocParams) -> Result<CallToolResult, McpError> {
    let response = store
        .get_document(
            params.doc_id.as_deref(),
            params.repo.as_deref(),
            params.path.as_deref(),
            params.chunk_range,
        )
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    json_success(&response)
}

pub async fn sync_sources(
    store: &DocStore,
    config: &KnowledgeConfig,
    params: SyncSourcesParams,
) -> Result<CallToolResult, McpError> {
    let response = ingest::run_sync(
        store,
        config,
        params.repo.as_deref(),
        params.source_name.as_deref(),
        params.path_prefix.as_deref(),
        params.force,
    )
    .await
    .map_err(|e| internal_error(e.to_string()))?;

    json_success(&response)
}

pub async fn get_sync_status(store: &DocStore) -> Result<CallToolResult, McpError> {
    let sources = store
        .get_sync_status()
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let response = SyncStatusResponse { sources };

    json_success(&response)
}

pub async fn list_sources(
    store: &DocStore,
    config: &KnowledgeConfig,
) -> Result<CallToolResult, McpError> {
    let mut sources = Vec::new();

    for source_cfg in &config.sources {
        let doc_count = store
            .get_source_doc_count(&source_cfg.name)
            .await
            .unwrap_or(0);

        sources.push(SourceInfo {
            name: source_cfg.name.clone(),
            repo: source_cfg.repo.clone(),
            base_path: source_cfg.base_path.clone(),
            patterns: source_cfg.patterns.clone(),
            exclude_patterns: source_cfg.exclude_patterns.clone(),
            enabled: source_cfg.enabled,
            document_count: doc_count,
        });
    }

    let response = ListSourcesResponse { sources };

    json_success(&response)
}

// ============================================================================
// Project Note Handlers
// ============================================================================

pub async fn list_projects(
    config: &KnowledgeConfig,
    params: ListProjectsParams,
) -> Result<CallToolResult, McpError> {
    let base = project_notes::project_notes_base(config)
        .map_err(|e| internal_error(e.to_string()))?;

    let mut projects = Vec::new();

    // Scan active project directories
    let mut entries = tokio::fs::read_dir(&base)
        .await
        .map_err(|e| internal_error(format!("Failed to read projects dir: {e}")))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| internal_error(e.to_string()))?
    {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if dir_name == "archived" || dir_name.starts_with('.') || dir_name.starts_with('_') {
            continue;
        }

        if let Some(entry) = read_project_entry(&path, &dir_name).await {
            projects.push(entry);
        }
    }

    // Include archived if requested
    if params.include_archived {
        let archived_dir = base.join("archived");
        if archived_dir.exists() {
            let mut archived_entries = tokio::fs::read_dir(&archived_dir)
                .await
                .map_err(|e| internal_error(format!("Failed to read archived dir: {e}")))?;

            while let Some(entry) = archived_entries
                .next_entry()
                .await
                .map_err(|e| internal_error(e.to_string()))?
            {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let dir_name = entry.file_name().to_string_lossy().to_string();
                if let Some(entry) = read_project_entry(&path, &dir_name).await {
                    projects.push(entry);
                }
            }
        }
    }

    // Filter by status if requested
    if let Some(ref status) = params.status {
        let s = status.to_lowercase();
        projects.retain(|p| p.status.to_lowercase() == s);
    }

    projects.sort_by(|a, b| a.name.cmp(&b.name));
    json_success(&ProjectListResponse { projects })
}

async fn read_project_entry(
    dir: &std::path::Path,
    dir_name: &str,
) -> Option<ProjectListEntry> {
    let summary_path = dir.join("summary.md");
    let content = tokio::fs::read_to_string(&summary_path).await.ok()?;
    let summary = project_notes::parse_summary(&content, dir_name);

    Some(ProjectListEntry {
        name: summary.name,
        status: summary.status,
        active_version: summary.active_version,
        repo: summary.repo,
        overview: summary.overview,
    })
}

pub async fn update_project_summary(
    store: &DocStore,
    config: &KnowledgeConfig,
    params: UpdateProjectSummaryParams,
) -> Result<CallToolResult, McpError> {
    let project_dir = project_notes::resolve_project_dir(config, &params.project)
        .map_err(|e| internal_error(e.to_string()))?;

    let summary_path = project_dir.join("summary.md");
    let mut content = tokio::fs::read_to_string(&summary_path)
        .await
        .map_err(|e| internal_error(format!("Failed to read summary.md: {e}")))?;

    let mut updated_fields = Vec::new();

    if let Some(ref status) = params.status {
        content = project_notes::replace_section(&content, "Status", &format!("{status}\n"));
        updated_fields.push("status".to_string());
    }

    if let Some(ref version) = params.active_version {
        let v = version.trim_start_matches('v');
        let link = format!("- [v{v}](v{v}.md)\n");
        content = project_notes::replace_section(&content, "Active Version", &link);
        updated_fields.push("active_version".to_string());
    }

    if let Some(ref overview) = params.overview {
        content = project_notes::replace_section(&content, "Overview", &format!("{overview}\n"));
        updated_fields.push("overview".to_string());
    }

    if let Some(ref repo) = params.repo {
        content = project_notes::replace_section(&content, "Repo", &format!("{repo}\n"));
        updated_fields.push("repo".to_string());
    }

    if let Some(ref notes) = params.notes {
        content = project_notes::append_to_section(&content, "Notes", &format!("- {notes}"));
        updated_fields.push("notes".to_string());
    }

    tokio::fs::write(&summary_path, &content)
        .await
        .map_err(|e| internal_error(format!("Failed to write summary.md: {e}")))?;

    // Re-index this project's files
    let _ = ingest::run_sync(
        store,
        config,
        None,
        Some("project-notes"),
        Some(&format!("{}/", params.project)),
        true,
    )
    .await;

    json_success(&ProjectUpdateResponse {
        project: params.project,
        updated_fields,
        file_path: summary_path.to_string_lossy().to_string(),
    })
}

pub async fn update_version(
    store: &DocStore,
    config: &KnowledgeConfig,
    params: UpdateVersionParams,
) -> Result<CallToolResult, McpError> {
    let project_dir = project_notes::resolve_project_dir(config, &params.project)
        .map_err(|e| internal_error(e.to_string()))?;

    let v = params.version.trim_start_matches('v');
    let version_filename = format!("v{v}.md");
    let version_path = project_dir.join(&version_filename);

    let mut tasks_added = 0;
    let mut tasks_toggled = 0;
    let created;

    if params.create {
        if version_path.exists() {
            return Err(internal_error(format!(
                "Version file already exists: {version_filename}"
            )));
        }

        let content =
            project_notes::create_version_content(v, params.description.as_deref());
        tokio::fs::write(&version_path, &content)
            .await
            .map_err(|e| internal_error(format!("Failed to create version file: {e}")))?;

        // Update summary.md active version
        let summary_path = project_dir.join("summary.md");
        if let Ok(summary_content) = tokio::fs::read_to_string(&summary_path).await {
            let link = format!("- [v{v}](v{v}.md)\n");
            let updated =
                project_notes::replace_section(&summary_content, "Active Version", &link);
            let _ = tokio::fs::write(&summary_path, &updated).await;
        }

        created = true;
    } else {
        if !version_path.exists() {
            return Err(internal_error(format!(
                "Version file not found: {version_filename}. Use create=true to create it."
            )));
        }

        let mut content = tokio::fs::read_to_string(&version_path)
            .await
            .map_err(|e| internal_error(format!("Failed to read version file: {e}")))?;

        if let Some(ref tasks) = params.add_tasks {
            content = project_notes::add_tasks(&content, tasks);
            tasks_added = tasks.len();
        }

        if let Some(ref check) = params.check_tasks {
            for task in check {
                content = project_notes::toggle_task(&content, task, true);
                tasks_toggled += 1;
            }
        }

        if let Some(ref uncheck) = params.uncheck_tasks {
            for task in uncheck {
                content = project_notes::toggle_task(&content, task, false);
                tasks_toggled += 1;
            }
        }

        tokio::fs::write(&version_path, &content)
            .await
            .map_err(|e| internal_error(format!("Failed to write version file: {e}")))?;

        created = false;
    }

    // Re-index
    let _ = ingest::run_sync(
        store,
        config,
        None,
        Some("project-notes"),
        Some(&format!("{}/", params.project)),
        true,
    )
    .await;

    json_success(&VersionUpdateResponse {
        project: params.project,
        version: format!("v{v}"),
        created,
        tasks_added,
        tasks_toggled,
        file_path: version_path.to_string_lossy().to_string(),
    })
}

