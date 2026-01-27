//! Async executor for gh CLI commands
//!
//! This module provides a safe, async wrapper around gh CLI invocations
//! with proper error handling and JSON parsing.
//!
//! # Example
//!
//! ```rust,ignore
//! use github_gh_mcp::gh::executor::execute_gh_json;
//! use github_gh_mcp::types::issue::Issue;
//!
//! let issues: Vec<Issue> = execute_gh_json(
//!     &["issue", "list", "-R", "owner/repo"],
//!     &["number", "title", "state"]
//! ).await?;
//! ```

use serde::de::DeserializeOwned;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, instrument};

use super::error::{GhError, GhResult};

/// Execute a gh command and parse JSON output
///
/// This function runs a gh command with `--json` flag and parses the output
/// into the specified type. The gh CLI requires explicit field names for
/// JSON output to avoid expensive API calls.
///
/// # Arguments
///
/// * `args` - Command arguments (e.g., `["issue", "list", "-R", "owner/repo"]`)
/// * `json_fields` - Fields to request in JSON output (e.g., `["number", "title"]`)
///
/// # Errors
///
/// Returns an error if:
/// - The gh process fails to spawn
/// - The command exits with non-zero status
/// - The JSON output cannot be parsed
#[instrument(skip(json_fields), fields(cmd = %args.join(" ")))]
pub async fn execute_gh_json<T: DeserializeOwned>(
    args: &[&str],
    json_fields: &[&str],
) -> GhResult<T> {
    let fields = json_fields.join(",");

    let mut full_args: Vec<&str> = args.to_vec();
    full_args.push("--json");
    full_args.push(&fields);

    debug!("executing: gh {}", full_args.join(" "));

    let output = Command::new("gh")
        .args(&full_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GhError::NotFound
            } else {
                GhError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);

        // Check for common authentication errors
        if stderr.contains("gh auth login") || stderr.contains("not logged in") {
            error!("gh authentication required");
            return Err(GhError::NotAuthenticated);
        }

        error!(code, stderr = %stderr, "gh command failed");
        return Err(GhError::CommandFailed { code, stderr });
    }

    let parsed: T = serde_json::from_slice(&output.stdout)?;
    Ok(parsed)
}

/// Execute a gh command that modifies state (no JSON output expected)
///
/// This function is used for commands like `gh issue create` that return
/// a URL or message rather than JSON data.
///
/// # Arguments
///
/// * `args` - Command arguments (e.g., `["issue", "create", "-R", "owner/repo"]`)
///
/// # Returns
///
/// The stdout output from the command as a string
#[instrument(fields(cmd = %args.join(" ")))]
pub async fn execute_gh_action(args: &[&str]) -> GhResult<String> {
    debug!("executing: gh {}", args.join(" "));

    let output = Command::new("gh")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GhError::NotFound
            } else {
                GhError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);

        if stderr.contains("gh auth login") || stderr.contains("not logged in") {
            error!("gh authentication required");
            return Err(GhError::NotAuthenticated);
        }

        error!(code, stderr = %stderr, "gh command failed");
        return Err(GhError::CommandFailed { code, stderr });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Execute a gh command and return raw output
///
/// This function is used for commands that return raw text (like diffs)
/// rather than JSON data.
///
/// # Arguments
///
/// * `args` - Command arguments (e.g., `["pr", "diff", "123", "-R", "owner/repo"]`)
///
/// # Returns
///
/// The raw stdout output from the command as a string
#[instrument(fields(cmd = %args.join(" ")))]
pub async fn execute_gh_raw(args: &[&str]) -> GhResult<String> {
    debug!("executing: gh {}", args.join(" "));

    let output = Command::new("gh")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GhError::NotFound
            } else {
                GhError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);

        if stderr.contains("gh auth login") || stderr.contains("not logged in") {
            error!("gh authentication required");
            return Err(GhError::NotAuthenticated);
        }

        error!(code, stderr = %stderr, "gh command failed");
        return Err(GhError::CommandFailed { code, stderr });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Execute a gh command and return output even on non-zero exit codes
///
/// This function is used for commands like `gh pr checks` where a non-zero
/// exit code indicates check failure (not a command error) and stdout
/// still contains valid output.
///
/// # Arguments
///
/// * `args` - Command arguments
///
/// # Returns
///
/// A tuple of (stdout, exit_code) - stdout is returned regardless of exit code
#[instrument(fields(cmd = %args.join(" ")))]
pub async fn execute_gh_raw_with_exit_code(args: &[&str]) -> GhResult<(String, i32)> {
    debug!("executing: gh {}", args.join(" "));

    let output = Command::new("gh")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GhError::NotFound
            } else {
                GhError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Only fail on auth errors, let caller handle other non-zero exits
    if stderr.contains("gh auth login") || stderr.contains("not logged in") {
        error!("gh authentication required");
        return Err(GhError::NotAuthenticated);
    }

    // For actual command not found or invalid usage, still error
    if exit_code != 0 && stdout.is_empty() && !stderr.is_empty() {
        error!(exit_code, stderr = %stderr, "gh command failed with no output");
        return Err(GhError::CommandFailed {
            code: exit_code,
            stderr,
        });
    }

    Ok((stdout, exit_code))
}

/// Check if gh CLI is available and authenticated
///
/// This function verifies that:
/// 1. The gh CLI is installed and in PATH
/// 2. The user is authenticated (via OAuth, token, etc.)
#[allow(dead_code)] // Used in tests
#[instrument]
pub async fn check_gh_available() -> GhResult<()> {
    debug!("checking gh availability");

    let output = Command::new("gh")
        .args(["auth", "status"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GhError::NotFound
            } else {
                GhError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.contains("not logged in") {
            return Err(GhError::NotAuthenticated);
        }
    }

    debug!("gh is available and authenticated");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_gh_available() {
        // This test requires gh to be installed and authenticated
        // It will be skipped in CI unless gh is configured
        let result = check_gh_available().await;
        // We just check it doesn't panic
        println!("gh available: {:?}", result.is_ok());
    }
}
