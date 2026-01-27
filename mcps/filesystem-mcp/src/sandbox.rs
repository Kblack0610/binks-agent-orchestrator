//! Sandbox module for path validation and security

use std::path::{Path, PathBuf};

use crate::types::{Config, FsError, FsResult};

/// Sandbox for filesystem operations with security controls
#[derive(Debug, Clone)]
pub struct Sandbox {
    /// Resolved read-allowed paths
    read_paths: Vec<PathBuf>,
    /// Resolved write-allowed paths
    write_paths: Vec<PathBuf>,
    /// Resolved denied paths
    deny_paths: Vec<PathBuf>,
    /// Home directory
    home_dir: PathBuf,
}

impl Sandbox {
    /// Create a new sandbox from configuration
    pub fn new(config: &Config) -> FsResult<Self> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            FsError::ConfigError("Could not determine home directory".to_string())
        })?;

        let read_paths = config
            .paths
            .read
            .iter()
            .filter_map(|p| Self::resolve_path_static(p, &home_dir))
            .collect();

        let write_paths = config
            .paths
            .write
            .iter()
            .filter_map(|p| Self::resolve_path_static(p, &home_dir))
            .collect();

        let deny_paths = config
            .paths
            .deny
            .iter()
            .filter_map(|p| Self::resolve_path_static(p, &home_dir))
            .collect();

        Ok(Self {
            read_paths,
            write_paths,
            deny_paths,
            home_dir,
        })
    }

    /// Resolve a path string to an absolute path
    fn resolve_path_static(path: &str, home_dir: &Path) -> Option<PathBuf> {
        let expanded = if let Some(stripped) = path.strip_prefix("~/") {
            home_dir.join(stripped)
        } else if path == "~" {
            home_dir.to_path_buf()
        } else {
            PathBuf::from(path)
        };

        // Try to canonicalize, but allow non-existent paths
        expanded.canonicalize().ok().or(Some(expanded))
    }

    /// Resolve a user-provided path to a canonical path
    pub fn resolve_path(&self, path: &str) -> FsResult<PathBuf> {
        // Reject paths containing null bytes (defense-in-depth)
        if path.contains('\0') {
            return Err(FsError::InvalidPath(
                "Path contains null byte".to_string(),
            ));
        }

        let expanded = if let Some(stripped) = path.strip_prefix("~/") {
            self.home_dir.join(stripped)
        } else if path == "~" {
            self.home_dir.clone()
        } else {
            PathBuf::from(path)
        };

        // Canonicalize to resolve symlinks and ..
        let canonical = expanded.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                // For non-existent paths, resolve what we can
                let mut resolved = PathBuf::new();
                for component in expanded.components() {
                    match component {
                        std::path::Component::ParentDir => {
                            resolved.pop();
                        }
                        std::path::Component::CurDir => {}
                        _ => resolved.push(component),
                    }
                }
                return FsError::NotFound(resolved.display().to_string());
            }
            FsError::InvalidPath(format!("{}: {}", expanded.display(), e))
        })?;

        Ok(canonical)
    }

    /// Check if a path is allowed for reading
    pub fn check_read(&self, path: &Path) -> FsResult<()> {
        // Check deny list first
        for deny in &self.deny_paths {
            if path.starts_with(deny) {
                return Err(FsError::AccessDenied(format!(
                    "Path {} is in deny list",
                    path.display()
                )));
            }
        }

        // Check if under any read path
        for allowed in &self.read_paths {
            if path.starts_with(allowed) {
                return Ok(());
            }
        }

        Err(FsError::AccessDenied(format!(
            "Path {} is not in allowlist",
            path.display()
        )))
    }

    /// Check if a path is allowed for writing
    pub fn check_write(&self, path: &Path) -> FsResult<()> {
        // Check deny list first
        for deny in &self.deny_paths {
            if path.starts_with(deny) {
                return Err(FsError::AccessDenied(format!(
                    "Path {} is in deny list",
                    path.display()
                )));
            }
        }

        // Check if under any write path
        for allowed in &self.write_paths {
            if path.starts_with(allowed) {
                return Ok(());
            }
        }

        Err(FsError::AccessDenied(format!(
            "Path {} is not writable",
            path.display()
        )))
    }

    /// Validate a path for reading, returning the canonical path
    pub fn validate_read(&self, path: &str) -> FsResult<PathBuf> {
        let canonical = self.resolve_path(path)?;
        self.check_read(&canonical)?;
        Ok(canonical)
    }

    /// Validate a path for writing, returning the canonical path
    /// For write operations on non-existent files, validates the parent directory
    pub fn validate_write(&self, path: &str) -> FsResult<PathBuf> {
        // Reject paths containing null bytes (defense-in-depth)
        if path.contains('\0') {
            return Err(FsError::InvalidPath(
                "Path contains null byte".to_string(),
            ));
        }

        let expanded = if let Some(stripped) = path.strip_prefix("~/") {
            self.home_dir.join(stripped)
        } else if path == "~" {
            self.home_dir.clone()
        } else {
            PathBuf::from(path)
        };

        // For new files, check the parent directory
        if !expanded.exists() {
            if let Some(parent) = expanded.parent() {
                let canonical_parent = parent
                    .canonicalize()
                    .map_err(|e| FsError::InvalidPath(format!("Parent directory: {}", e)))?;
                self.check_write(&canonical_parent)?;
                // Return the full intended path
                return Ok(canonical_parent.join(
                    expanded
                        .file_name()
                        .ok_or_else(|| FsError::InvalidPath("No filename".to_string()))?,
                ));
            }
        }

        let canonical = self.resolve_path(path)?;
        self.check_write(&canonical)?;
        Ok(canonical)
    }

    /// Get allowed read paths for listing
    pub fn allowed_read_paths(&self) -> Vec<String> {
        self.read_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect()
    }

    /// Get allowed write paths
    pub fn allowed_write_paths(&self) -> Vec<String> {
        self.write_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PathConfig;

    fn test_config() -> Config {
        Config {
            paths: PathConfig {
                read: vec!["/tmp".to_string()],
                write: vec!["/tmp".to_string()],
                deny: vec!["/tmp/secret".to_string()],
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_read_allowed() {
        let sandbox = Sandbox::new(&test_config()).unwrap();
        let path = PathBuf::from("/tmp");
        assert!(sandbox.check_read(&path).is_ok());
    }

    #[test]
    fn test_read_denied() {
        let sandbox = Sandbox::new(&test_config()).unwrap();
        let path = PathBuf::from("/etc/passwd");
        assert!(sandbox.check_read(&path).is_err());
    }

    #[test]
    fn test_deny_list() {
        let sandbox = Sandbox::new(&test_config()).unwrap();
        let path = PathBuf::from("/tmp/secret/file.txt");
        assert!(sandbox.check_read(&path).is_err());
        assert!(sandbox.check_write(&path).is_err());
    }
}
