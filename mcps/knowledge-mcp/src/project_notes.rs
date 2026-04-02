//! Project notes parsing, writing, and path resolution.
//!
//! Operates on `~/.notes/dev/projects/{project}/` structure:
//! - `summary.md` — YAML frontmatter + sections (Overview, Status, Active Version, Repo, Notes)
//! - `v{X.Y.Z}.md` — version checklists with `- [ ]` / `- [x]` tasks
//! - `changelog.md` — Keep-a-Changelog format

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::KnowledgeConfig;
use crate::types::{ChangelogEntry, KnowledgeError, ProjectSummary};

// ============================================================================
// Path Resolution
// ============================================================================

/// Get the base path for project notes from the writable "project-notes" source.
pub fn project_notes_base(config: &KnowledgeConfig) -> Result<PathBuf, KnowledgeError> {
    let source = config
        .get_writable_source("project-notes")
        .ok_or_else(|| {
            KnowledgeError::Config(
                "No writable source named 'project-notes' found in config".into(),
            )
        })?;
    let base = PathBuf::from(&source.base_path);
    if !base.exists() {
        return Err(KnowledgeError::NotFound(format!(
            "Project notes base path does not exist: {}",
            base.display()
        )));
    }
    Ok(base)
}

/// Resolve a specific project directory within the notes base.
pub fn resolve_project_dir(
    config: &KnowledgeConfig,
    project: &str,
) -> Result<PathBuf, KnowledgeError> {
    let base = project_notes_base(config)?;
    let dir = base.join(project);
    if !dir.exists() {
        return Err(KnowledgeError::NotFound(format!(
            "Project directory not found: {}",
            dir.display()
        )));
    }
    Ok(dir)
}

/// Resolve a repo's local path by cross-referencing knowledge sources.
///
/// `repo_field` is the content of `## Repo` in summary.md, e.g. `kblack0610/dodginballs`
/// or `BlackNBrownStudios/platform (apps/placemyparents)`.
/// Extracts the repo name and looks for a matching knowledge source.
pub fn resolve_repo_path(
    config: &KnowledgeConfig,
    repo_field: &str,
) -> Option<PathBuf> {
    let trimmed = repo_field.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Extract owner/repo, ignoring any parenthetical suffix
    let owner_repo = trimmed.split_whitespace().next()?;
    // Extract just the repo name (after /)
    let repo_name = owner_repo.rsplit('/').next()?;

    config
        .find_source_by_repo(repo_name)
        .map(|s| PathBuf::from(&s.base_path))
}

// ============================================================================
// Summary Parsing
// ============================================================================

/// Parse a summary.md file into structured fields.
pub fn parse_summary(content: &str, project_name: &str) -> ProjectSummary {
    let body = skip_frontmatter(content);
    let sections = parse_sections(&body);

    let name = sections
        .get("_title")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| project_name.to_string());

    let overview = sections
        .get("Overview")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let status = sections
        .get("Status")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let active_version = sections
        .get("Active Version")
        .and_then(|s| parse_version_link(s).or_else(|| {
            let t = s.trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        }));

    let repo = sections
        .get("Repo")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let notes = sections
        .get("Notes")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    ProjectSummary {
        name,
        overview,
        status,
        active_version,
        repo,
        notes,
    }
}

/// Extract version string from a markdown link like `- [v1.6.0](v1.6.0.md)`
fn parse_version_link(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim().trim_start_matches('-').trim();
        if let Some(rest) = trimmed.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

/// Skip YAML frontmatter (content between --- delimiters).
fn skip_frontmatter(content: &str) -> String {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return content.to_string();
    }

    // Find the closing ---
    if let Some(rest) = trimmed.strip_prefix("---") {
        if let Some(end_idx) = rest.find("\n---") {
            // Skip past the closing --- and its newline
            let after = &rest[end_idx + 4..];
            return after.to_string();
        }
    }

    content.to_string()
}

/// Parse markdown content into sections keyed by `## Heading`.
/// `_title` key holds the `# Title` content.
fn parse_sections(content: &str) -> BTreeMap<String, String> {
    let mut sections = BTreeMap::new();
    let mut current_key: Option<String> = None;
    let mut current_content = String::new();

    for line in content.lines() {
        if let Some(title) = line.strip_prefix("# ") {
            if !title.starts_with('#') {
                // Top-level heading
                sections.insert("_title".to_string(), title.trim().to_string());
                continue;
            }
        }

        if let Some(heading) = line.strip_prefix("## ") {
            // Save previous section
            if let Some(key) = current_key.take() {
                sections.insert(key, current_content.clone());
            }
            current_key = Some(heading.trim().to_string());
            current_content.clear();
        } else if current_key.is_some() {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    // Save last section
    if let Some(key) = current_key {
        sections.insert(key, current_content);
    }

    sections
}

// ============================================================================
// Summary Writing
// ============================================================================

/// Replace the body of a `## Heading` section in markdown content.
/// Preserves frontmatter and all other sections.
pub fn replace_section(content: &str, heading: &str, new_body: &str) -> String {
    let target = format!("## {heading}");
    let mut result = String::new();
    let mut in_target = false;
    let mut replaced = false;

    for line in content.lines() {
        if line.starts_with("## ") {
            if line.trim() == target {
                in_target = true;
                replaced = true;
                result.push_str(line);
                result.push('\n');
                result.push_str(new_body);
                if !new_body.ends_with('\n') {
                    result.push('\n');
                }
                continue;
            } else if in_target {
                in_target = false;
            }
        }

        if !in_target {
            result.push_str(line);
            result.push('\n');
        }
    }

    // If section wasn't found, append it
    if !replaced {
        if !result.ends_with('\n') {
            result.push('\n');
        }
        result.push_str(&format!("\n{target}\n{new_body}\n"));
    }

    result
}

/// Append a line to the end of a `## Heading` section.
pub fn append_to_section(content: &str, heading: &str, line: &str) -> String {
    let target = format!("## {heading}");
    let mut result = String::new();
    let mut in_target = false;
    let mut appended = false;

    for content_line in content.lines() {
        if content_line.starts_with("## ") {
            if in_target {
                // Insert before the next heading
                result.push_str(line);
                result.push('\n');
                appended = true;
                in_target = false;
            }
            if content_line.trim() == target {
                in_target = true;
            }
        }
        result.push_str(content_line);
        result.push('\n');
    }

    // If we were still in the target section at EOF
    if in_target && !appended {
        result.push_str(line);
        result.push('\n');
    }

    result
}

// ============================================================================
// Version Files
// ============================================================================

/// Create content for a new version file.
pub fn create_version_content(version: &str, description: Option<&str>) -> String {
    let date = chrono::Local::now().format("%Y-%m-%d");
    let desc = description.unwrap_or("");
    if desc.is_empty() {
        format!("# v{version} - {date}\n\n- [ ] \n")
    } else {
        format!("# v{version} - {date} {desc}\n\n- [ ] \n")
    }
}

/// Toggle a task checkbox. Matches by substring of task text.
pub fn toggle_task(content: &str, task_text: &str, checked: bool) -> String {
    let mut result = String::new();
    let search = task_text.trim().to_lowercase();

    for line in content.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        if (lower.contains("- [ ]") || lower.contains("- [x]")) && lower.contains(&search) {
            if checked {
                result.push_str(&line.replace("- [ ]", "- [x]"));
            } else {
                result.push_str(&line.replace("- [x]", "- [ ]"));
            }
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    result
}

/// Append new unchecked tasks at the end of a version file.
pub fn add_tasks(content: &str, tasks: &[String]) -> String {
    let mut result = content.trim_end().to_string();
    result.push('\n');
    for task in tasks {
        result.push_str(&format!("\n- [ ] {task}"));
    }
    result.push('\n');
    result
}

// ============================================================================
// Changelog
// ============================================================================

/// Format a changelog entry in Keep-a-Changelog style.
pub fn format_changelog_entry(version: &str, entries: &[ChangelogEntry]) -> String {
    let date = chrono::Local::now().format("%Y-%m-%d");
    let mut result = format!("## [{version}] - {date}\n");

    // Group entries by category
    let mut by_category: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in entries {
        by_category
            .entry(entry.category.clone())
            .or_default()
            .push(entry.description.clone());
    }

    for (category, descriptions) in &by_category {
        result.push_str(&format!("\n### {category}\n"));
        for desc in descriptions {
            result.push_str(&format!("- {desc}\n"));
        }
    }

    result
}

/// Prepend a changelog entry after the `# Changelog` header.
pub fn prepend_changelog_entry(existing: &str, new_entry: &str) -> String {
    let trimmed = existing.trim();

    if trimmed.is_empty() {
        return format!("# Changelog\n\n{new_entry}");
    }

    // Find the first ## heading (existing entry) and insert before it
    if let Some(pos) = trimmed.find("\n## ") {
        let (header, rest) = trimmed.split_at(pos + 1);
        format!("{header}\n{new_entry}\n{rest}")
    } else {
        // No existing entries, just append
        format!("{trimmed}\n\n{new_entry}")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SUMMARY: &str = r#"---
id: summary
aliases: []
tags: []
---

# Dodginballs

## Overview
A production game with 3 modes

## Status
active

## Active Version
- [v1.0.0](v1.0.0.md)

## Repo
kblack0610/dodginballs

## Notes
v0.0.5 demo ready
"#;

    #[test]
    fn test_parse_summary() {
        let summary = parse_summary(SAMPLE_SUMMARY, "dodginballs");
        assert_eq!(summary.name, "Dodginballs");
        assert_eq!(summary.overview, "A production game with 3 modes");
        assert_eq!(summary.status, "active");
        assert_eq!(summary.active_version.as_deref(), Some("v1.0.0"));
        assert_eq!(summary.repo.as_deref(), Some("kblack0610/dodginballs"));
        assert!(summary.notes.contains("v0.0.5 demo ready"));
    }

    #[test]
    fn test_parse_version_link() {
        assert_eq!(
            parse_version_link("- [v1.6.0](v1.6.0.md)"),
            Some("v1.6.0".to_string())
        );
        assert_eq!(parse_version_link(""), None);
    }

    #[test]
    fn test_skip_frontmatter() {
        let content = "---\nid: test\n---\n\n# Title\n";
        let result = skip_frontmatter(content);
        assert!(result.contains("# Title"));
        assert!(!result.contains("id: test"));
    }

    #[test]
    fn test_replace_section() {
        let result = replace_section(SAMPLE_SUMMARY, "Status", "paused\n");
        assert!(result.contains("## Status\npaused\n"));
        assert!(result.contains("## Overview"));
    }

    #[test]
    fn test_append_to_section() {
        let result = append_to_section(SAMPLE_SUMMARY, "Notes", "- new note\n");
        assert!(result.contains("v0.0.5 demo ready"));
        assert!(result.contains("- new note"));
    }

    #[test]
    fn test_toggle_task() {
        let content = "- [ ] single player core mode\n- [x] school only\n";
        let result = toggle_task(content, "single player", true);
        assert!(result.contains("- [x] single player core mode"));

        let result = toggle_task(content, "school", false);
        assert!(result.contains("- [ ] school only"));
    }

    #[test]
    fn test_add_tasks() {
        let content = "# v1.0.0\n\n- [ ] existing task\n";
        let result = add_tasks(content, &["new task 1".into(), "new task 2".into()]);
        assert!(result.contains("- [ ] new task 1"));
        assert!(result.contains("- [ ] new task 2"));
    }

    #[test]
    fn test_format_changelog_entry() {
        let entries = vec![
            ChangelogEntry {
                category: "Added".into(),
                description: "New feature".into(),
            },
            ChangelogEntry {
                category: "Fixed".into(),
                description: "Bug fix".into(),
            },
        ];
        let result = format_changelog_entry("1.0.0", &entries);
        assert!(result.contains("## [1.0.0]"));
        assert!(result.contains("### Added"));
        assert!(result.contains("- New feature"));
        assert!(result.contains("### Fixed"));
        assert!(result.contains("- Bug fix"));
    }

    #[test]
    fn test_prepend_changelog_entry() {
        let existing = "# Changelog\n\n## [0.9.0] - 2025-01-01\n\n### Added\n- Old feature\n";
        let new_entry = "## [1.0.0] - 2025-06-01\n\n### Added\n- New feature\n";
        let result = prepend_changelog_entry(existing, new_entry);
        // New entry should come before old entry
        let new_pos = result.find("[1.0.0]").unwrap();
        let old_pos = result.find("[0.9.0]").unwrap();
        assert!(new_pos < old_pos);
    }

    #[test]
    fn test_prepend_changelog_entry_empty() {
        let result = prepend_changelog_entry("", "## [1.0.0] - 2025-06-01\n");
        assert!(result.starts_with("# Changelog"));
        assert!(result.contains("[1.0.0]"));
    }

    #[test]
    fn test_create_version_content() {
        let result = create_version_content("2.0.0", Some("big release"));
        assert!(result.contains("# v2.0.0"));
        assert!(result.contains("big release"));
        assert!(result.contains("- [ ]"));
    }
}
