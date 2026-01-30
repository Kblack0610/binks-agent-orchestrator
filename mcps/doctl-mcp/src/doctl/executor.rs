//! Async executor for doctl CLI commands
//!
//! This module provides a safe, async wrapper around doctl CLI invocations
//! with proper error handling and JSON parsing.
//!
//! # Example
//!
//! ```rust,ignore
//! use doctl_mcp::doctl::executor::execute_doctl_json;
//! use serde_json::Value;
//!
//! let droplets: Vec<Value> = execute_doctl_json(
//!     &["compute", "droplet", "list"],
//! ).await?;
//! ```

use serde::de::DeserializeOwned;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, instrument};

use super::error::{DoctlError, DoctlResult};

/// Execute a doctl command and parse JSON output
///
/// Appends `--output json` to the command arguments and parses the result.
///
/// # Arguments
///
/// * `args` - Command arguments (e.g., `["compute", "droplet", "list"]`)
///
/// # Errors
///
/// Returns an error if:
/// - The doctl process fails to spawn
/// - The command exits with non-zero status
/// - The JSON output cannot be parsed
#[instrument(fields(cmd = %args.join(" ")))]
pub async fn execute_doctl_json<T: DeserializeOwned>(args: &[&str]) -> DoctlResult<T> {
    let mut full_args: Vec<&str> = args.to_vec();
    full_args.push("--output");
    full_args.push("json");

    debug!("executing: doctl {}", full_args.join(" "));

    let output = Command::new("doctl")
        .args(&full_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DoctlError::NotFound
            } else {
                DoctlError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);

        if stderr.contains("Unable to authenticate")
            || stderr.contains("doctl auth init")
            || stderr.contains("unauthorized")
        {
            error!("doctl authentication required");
            return Err(DoctlError::NotAuthenticated);
        }

        error!(code, stderr = %stderr, "doctl command failed");
        return Err(DoctlError::CommandFailed { code, stderr });
    }

    let parsed: T = serde_json::from_slice(&output.stdout)?;
    Ok(parsed)
}

/// Execute a doctl command that modifies state (no JSON output expected)
///
/// Used for commands like `doctl compute droplet delete` that return
/// a message rather than structured JSON data.
///
/// # Arguments
///
/// * `args` - Command arguments (e.g., `["compute", "droplet", "delete", "12345", "--force"]`)
///
/// # Returns
///
/// The stdout output from the command as a string
#[instrument(fields(cmd = %args.join(" ")))]
pub async fn execute_doctl_action(args: &[&str]) -> DoctlResult<String> {
    debug!("executing: doctl {}", args.join(" "));

    let output = Command::new("doctl")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DoctlError::NotFound
            } else {
                DoctlError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);

        if stderr.contains("Unable to authenticate")
            || stderr.contains("doctl auth init")
            || stderr.contains("unauthorized")
        {
            error!("doctl authentication required");
            return Err(DoctlError::NotAuthenticated);
        }

        error!(code, stderr = %stderr, "doctl command failed");
        return Err(DoctlError::CommandFailed { code, stderr });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Execute a doctl command and return raw output
///
/// Used for commands that return raw text rather than JSON data.
///
/// # Arguments
///
/// * `args` - Command arguments
///
/// # Returns
///
/// The raw stdout output from the command as a string
#[instrument(fields(cmd = %args.join(" ")))]
pub async fn execute_doctl_raw(args: &[&str]) -> DoctlResult<String> {
    debug!("executing: doctl {}", args.join(" "));

    let output = Command::new("doctl")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DoctlError::NotFound
            } else {
                DoctlError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);

        if stderr.contains("Unable to authenticate")
            || stderr.contains("doctl auth init")
            || stderr.contains("unauthorized")
        {
            error!("doctl authentication required");
            return Err(DoctlError::NotAuthenticated);
        }

        error!(code, stderr = %stderr, "doctl command failed");
        return Err(DoctlError::CommandFailed { code, stderr });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Check if doctl CLI is available and authenticated
///
/// Verifies that:
/// 1. The doctl CLI is installed and in PATH
/// 2. The user is authenticated
#[allow(dead_code)]
#[instrument]
pub async fn check_doctl_available() -> DoctlResult<()> {
    debug!("checking doctl availability");

    let output = Command::new("doctl")
        .args(["account", "get", "--output", "json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DoctlError::NotFound
            } else {
                DoctlError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.contains("Unable to authenticate")
            || stderr.contains("doctl auth init")
            || stderr.contains("unauthorized")
        {
            return Err(DoctlError::NotAuthenticated);
        }
    }

    debug!("doctl is available and authenticated");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_doctl_available() {
        let result = check_doctl_available().await;
        println!("doctl available: {:?}", result.is_ok());
    }
}
