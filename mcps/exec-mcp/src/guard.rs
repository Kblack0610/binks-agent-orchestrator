//! Command guard - safety layer for command execution
//!
//! Validates commands against allow/deny lists and enforces
//! working directory restrictions.

use std::path::PathBuf;

use regex::Regex;

use crate::types::{Config, ExecError};

/// Command execution guard with allow/deny list enforcement
#[derive(Clone)]
pub struct CommandGuard {
    deny_patterns: Vec<Regex>,
    allow_patterns: Vec<Regex>,
    allowed_dirs: Vec<PathBuf>,
    shell: String,
}

impl CommandGuard {
    /// Create a new CommandGuard from config
    pub fn new(config: &Config) -> Result<Self, ExecError> {
        let deny_patterns = config
            .commands
            .deny_patterns
            .iter()
            .map(|p| {
                Regex::new(p).map_err(|e| {
                    ExecError::ConfigError(format!("Invalid deny pattern '{}': {}", p, e))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let allow_patterns = config
            .commands
            .allow_patterns
            .iter()
            .map(|p| {
                Regex::new(p).map_err(|e| {
                    ExecError::ConfigError(format!("Invalid allow pattern '{}': {}", p, e))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let allowed_dirs = config
            .commands
            .allowed_dirs
            .iter()
            .map(|d| resolve_path(d))
            .collect();

        Ok(Self {
            deny_patterns,
            allow_patterns,
            allowed_dirs,
            shell: config.commands.shell.clone(),
        })
    }

    /// Check if a command is allowed
    pub fn check_command(&self, command: &str) -> Result<(), ExecError> {
        // Deny list always takes precedence
        for pattern in &self.deny_patterns {
            if pattern.is_match(command) {
                return Err(ExecError::CommandDenied(format!(
                    "Command matches deny pattern: {}",
                    pattern.as_str()
                )));
            }
        }

        // If allow patterns are configured, command must match at least one
        if !self.allow_patterns.is_empty() {
            let allowed = self.allow_patterns.iter().any(|p| p.is_match(command));
            if !allowed {
                return Err(ExecError::CommandDenied(
                    "Command does not match any allow pattern".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Validate and resolve working directory
    pub fn validate_cwd(&self, cwd: Option<&str>) -> Result<PathBuf, ExecError> {
        let resolved = match cwd {
            Some(dir) => {
                let path = resolve_path(dir);
                let canonical = path
                    .canonicalize()
                    .map_err(|e| ExecError::DirNotAllowed(format!("{}: {}", dir, e)))?;
                canonical
            }
            None => {
                // Default to home directory
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
            }
        };

        // Check against allowed directories
        let is_allowed = self.allowed_dirs.iter().any(|allowed| {
            let Ok(allowed_canonical) = allowed.canonicalize() else {
                return false;
            };
            resolved.starts_with(&allowed_canonical)
        });

        if !is_allowed {
            return Err(ExecError::DirNotAllowed(format!(
                "{} is not within allowed directories",
                resolved.display()
            )));
        }

        Ok(resolved)
    }

    /// Get the configured shell path
    pub fn shell(&self) -> &str {
        &self.shell
    }
}

/// Resolve ~ to home directory
fn resolve_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest.trim_start_matches('/'));
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Config;

    #[test]
    fn test_deny_patterns_block_dangerous() {
        let config = Config::default();
        let guard = CommandGuard::new(&config).unwrap();

        assert!(guard.check_command("rm -rf /").is_err());
        assert!(guard.check_command("mkfs.ext4 /dev/sda1").is_err());
        assert!(guard.check_command("shutdown -h now").is_err());
        assert!(guard.check_command("reboot").is_err());
    }

    #[test]
    fn test_safe_commands_allowed() {
        let config = Config::default();
        let guard = CommandGuard::new(&config).unwrap();

        assert!(guard.check_command("ls -la").is_ok());
        assert!(guard.check_command("cat /tmp/foo.txt").is_ok());
        assert!(guard.check_command("echo hello").is_ok());
        assert!(guard.check_command("cargo build").is_ok());
        assert!(guard.check_command("git status").is_ok());
    }

    #[test]
    fn test_tilde_resolution() {
        let resolved = resolve_path("~/dev");
        assert!(resolved.to_string_lossy().contains("/dev"));
        assert!(!resolved.to_string_lossy().starts_with('~'));
    }
}
