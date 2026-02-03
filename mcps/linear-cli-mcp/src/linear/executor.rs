//! Async executor for linear CLI commands
//!
//! Provides safe, async wrappers around the `linear` CLI with
//! proper error handling and optional JSON parsing.

use serde::de::DeserializeOwned;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, instrument};

use super::error::{LinearError, LinearResult};

/// Execute a linear command and return stdout as text
///
/// Used for most commands that return formatted text output.
#[instrument(fields(cmd = %args.join(" ")))]
pub async fn execute_linear(args: &[&str]) -> LinearResult<String> {
    debug!("executing: linear {}", args.join(" "));

    let output = Command::new("linear")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                LinearError::NotFound
            } else {
                LinearError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);
        error!(code, stderr = %stderr, "linear command failed");
        return Err(LinearError::CommandFailed { code, stderr });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Execute a linear command with `--json` flag and parse output
///
/// Used for commands that support JSON output (primarily document commands).
#[instrument(fields(cmd = %args.join(" ")))]
pub async fn execute_linear_json<T: DeserializeOwned>(args: &[&str]) -> LinearResult<T> {
    let mut full_args: Vec<&str> = args.to_vec();
    full_args.push("--json");

    debug!("executing: linear {}", full_args.join(" "));

    let output = Command::new("linear")
        .args(&full_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                LinearError::NotFound
            } else {
                LinearError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);
        error!(code, stderr = %stderr, "linear command failed");
        return Err(LinearError::CommandFailed { code, stderr });
    }

    let parsed: T = serde_json::from_slice(&output.stdout)?;
    Ok(parsed)
}

/// Check if linear CLI is available
#[allow(dead_code)]
#[instrument]
pub async fn check_linear_available() -> LinearResult<()> {
    debug!("checking linear CLI availability");

    let output = Command::new("linear")
        .args(["--version"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                LinearError::NotFound
            } else {
                LinearError::SpawnError(e)
            }
        })?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(LinearError::CommandFailed {
            code: output.status.code().unwrap_or(-1),
            stderr,
        });
    }

    debug!("linear CLI is available");
    Ok(())
}
