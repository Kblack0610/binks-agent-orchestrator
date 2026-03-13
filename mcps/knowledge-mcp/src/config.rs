//! Configuration loading for knowledge-mcp
//!
//! Reads from `~/.binks/knowledge.toml` (or KNOWLEDGE_CONFIG_PATH env var).
//! Fails with a clear error if the config file is missing.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::types::KnowledgeError;

/// Top-level configuration
#[derive(Debug, Clone, Deserialize)]
pub struct KnowledgeConfig {
    pub database: DatabaseConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            max_file_size: default_max_file_size(),
        }
    }
}

fn default_max_file_size() -> u64 {
    1_048_576 // 1MB
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceConfig {
    pub name: String,
    pub repo: String,
    pub base_path: String,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    #[serde(default)]
    pub kind_map: HashMap<String, String>,
    #[serde(default)]
    pub priority_map: HashMap<String, i32>,
    /// Default kind for all files in this source (overridden by kind_map)
    pub kind: Option<String>,
    /// Default priority for all files in this source (overridden by priority_map)
    pub priority: Option<i32>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl KnowledgeConfig {
    /// Load configuration from file.
    ///
    /// Resolution order:
    /// 1. `KNOWLEDGE_CONFIG_PATH` env var
    /// 2. `~/.binks/knowledge.toml`
    ///
    /// Fails with a clear error if missing.
    pub fn load() -> Result<Self, KnowledgeError> {
        let config_path = Self::resolve_config_path()?;

        let contents = std::fs::read_to_string(&config_path).map_err(|e| {
            KnowledgeError::Config(format!(
                "Cannot read config at {}: {}. \
                 Copy knowledge.example.toml to {} and adjust paths.",
                config_path.display(),
                e,
                config_path.display()
            ))
        })?;

        let mut config: KnowledgeConfig = toml::from_str(&contents).map_err(|e| {
            KnowledgeError::Config(format!(
                "Invalid config at {}: {}",
                config_path.display(),
                e
            ))
        })?;

        // Expand ~ in all paths
        config.database.path = expand_tilde(&config.database.path);
        for source in &mut config.sources {
            source.base_path = expand_tilde(&source.base_path);
        }

        Ok(config)
    }

    fn resolve_config_path() -> Result<PathBuf, KnowledgeError> {
        if let Ok(path) = std::env::var("KNOWLEDGE_CONFIG_PATH") {
            let p = PathBuf::from(expand_tilde(&path));
            if p.exists() {
                return Ok(p);
            }
            return Err(KnowledgeError::Config(format!(
                "KNOWLEDGE_CONFIG_PATH set to {} but file does not exist",
                p.display()
            )));
        }

        let default_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".binks")
            .join("knowledge.toml");

        if default_path.exists() {
            return Ok(default_path);
        }

        Err(KnowledgeError::Config(format!(
            "Config file not found at {}. \
             Copy knowledge.example.toml to that path and adjust for your environment.",
            default_path.display()
        )))
    }

    /// Get the resolved database path
    pub fn db_path(&self) -> PathBuf {
        PathBuf::from(&self.database.path)
    }
}

/// Expand `~` at the start of a path to the user's home directory
pub fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().to_string();
        }
    }
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home.to_string_lossy().to_string();
        }
    }
    path.to_string()
}

/// Match a relative path against a glob pattern
pub fn path_matches_glob(pattern: &str, rel_path: &str) -> bool {
    match glob::Pattern::new(pattern) {
        Ok(pat) => pat.matches(rel_path),
        Err(_) => false,
    }
}

/// Determine the `kind` for a file based on kind_map, source default, or fallback
pub fn resolve_kind(source: &SourceConfig, rel_path: &str) -> String {
    // Check kind_map patterns in order
    for (pattern, kind) in &source.kind_map {
        if path_matches_glob(pattern, rel_path) {
            return kind.clone();
        }
    }
    // Source-level default
    if let Some(ref kind) = source.kind {
        return kind.clone();
    }
    // Infer from filename
    infer_kind_from_filename(Path::new(rel_path))
}

/// Determine the `priority` for a file based on priority_map, source default, or 0
pub fn resolve_priority(source: &SourceConfig, rel_path: &str) -> i32 {
    // Check priority_map patterns
    for (pattern, priority) in &source.priority_map {
        if path_matches_glob(pattern, rel_path) {
            return *priority;
        }
    }
    // Source-level default
    source.priority.unwrap_or(0)
}

/// Infer document kind from filename
fn infer_kind_from_filename(path: &Path) -> String {
    let filename = path
        .file_name()
        .map(|f| f.to_string_lossy().to_uppercase())
        .unwrap_or_default();

    match filename.as_str() {
        "CLAUDE.MD" | "AGENTS.MD" => "instruction".to_string(),
        "CONTEXT.MD" => "architecture".to_string(),
        "README.MD" => "docs".to_string(),
        "ARCHITECTURE.MD" => "architecture".to_string(),
        "ROADMAP.MD" => "architecture".to_string(),
        _ => {
            // Check path components
            let path_str = path.to_string_lossy().to_lowercase();
            if path_str.contains("runbook") || path_str.contains("infra") {
                "runbook".to_string()
            } else if path_str.contains("plan") {
                "plan".to_string()
            } else if path_str.contains("lesson") {
                "lesson".to_string()
            } else {
                "docs".to_string()
            }
        }
    }
}
