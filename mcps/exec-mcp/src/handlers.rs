//! Command execution handlers
//!
//! Each handler validates the command through the guard, then executes it
//! with timeout enforcement and output size limits.

use std::process::Stdio;

use mcp_common::{internal_error, json_success, CallToolResult, McpError};
use tokio::process::Command;

use crate::guard::CommandGuard;
use crate::params::*;
use crate::types::{CommandOutput, Config, ExecError};

// ============================================================================
// Helper Functions
// ============================================================================

fn exec_error_to_mcp(err: ExecError) -> McpError {
    match &err {
        ExecError::CommandDenied(_) | ExecError::DirNotAllowed(_) => {
            McpError::invalid_request(err.to_string(), None)
        }
        ExecError::Timeout(_) => internal_error(err.to_string()),
        ExecError::ConfigError(_) => internal_error(err.to_string()),
        ExecError::IoError(_) => internal_error(err.to_string()),
    }
}

/// Truncate output to max bytes on a UTF-8 boundary
fn truncate_output(output: &[u8], max_bytes: usize) -> (String, bool) {
    if output.len() <= max_bytes {
        let text = String::from_utf8_lossy(output).to_string();
        (text, false)
    } else {
        let text = String::from_utf8_lossy(&output[..max_bytes]).to_string();
        (text, true)
    }
}

/// Core command execution logic
async fn execute(
    guard: &CommandGuard,
    config: &Config,
    command: &str,
    cwd: Option<&str>,
    timeout_secs: u64,
) -> Result<CommandOutput, ExecError> {
    // 1. Validate command against allow/deny lists
    guard.check_command(command)?;

    // 2. Validate and resolve working directory
    let working_dir = guard.validate_cwd(cwd)?;

    // 3. Build the command
    let mut cmd = Command::new(guard.shell());
    cmd.arg("-c")
        .arg(command)
        .current_dir(&working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Apply environment config
    for (key, value) in &config.environment.set {
        cmd.env(key, value);
    }
    for key in &config.environment.remove {
        cmd.env_remove(key);
    }

    // 4. Execute with timeout
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let result = tokio::time::timeout(timeout, cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            let (stdout, stdout_truncated) =
                truncate_output(&output.stdout, config.limits.max_output_bytes);
            let (stderr, stderr_truncated) =
                truncate_output(&output.stderr, config.limits.max_output_bytes);

            Ok(CommandOutput {
                command: command.to_string(),
                exit_code: output.status.code(),
                stdout,
                stderr,
                timed_out: false,
                truncated: stdout_truncated || stderr_truncated,
            })
        }
        Ok(Err(io_err)) => Err(ExecError::IoError(io_err)),
        Err(_elapsed) => {
            // Timeout - the child process is dropped (killed) automatically
            Err(ExecError::Timeout(timeout_secs))
        }
    }
}

// ============================================================================
// Handler Functions
// ============================================================================

pub async fn run_command(
    guard: &CommandGuard,
    config: &Config,
    params: RunCommandParams,
) -> Result<CallToolResult, McpError> {
    let timeout = config.timeouts.default_secs;

    let output = execute(
        guard,
        config,
        &params.command,
        params.cwd.as_deref(),
        timeout,
    )
    .await
    .map_err(exec_error_to_mcp)?;

    json_success(&output)
}

pub async fn run_command_with_timeout(
    guard: &CommandGuard,
    config: &Config,
    params: RunCommandWithTimeoutParams,
) -> Result<CallToolResult, McpError> {
    // Clamp to server max
    let timeout = params.timeout_secs.min(config.timeouts.max_secs);

    let output = execute(
        guard,
        config,
        &params.command,
        params.cwd.as_deref(),
        timeout,
    )
    .await
    .map_err(exec_error_to_mcp)?;

    json_success(&output)
}

pub async fn run_script(
    guard: &CommandGuard,
    config: &Config,
    params: RunScriptParams,
) -> Result<CallToolResult, McpError> {
    let timeout = params
        .timeout_secs
        .unwrap_or(config.timeouts.default_secs)
        .min(config.timeouts.max_secs);

    // For scripts, validate the whole script text against deny patterns
    guard
        .check_command(&params.script)
        .map_err(exec_error_to_mcp)?;

    // Validate working directory
    let working_dir = guard
        .validate_cwd(params.cwd.as_deref())
        .map_err(exec_error_to_mcp)?;

    // Build command - pass script via stdin-like -c
    let mut cmd = Command::new(guard.shell());
    cmd.arg("-c")
        .arg(&params.script)
        .current_dir(&working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Apply environment config
    for (key, value) in &config.environment.set {
        cmd.env(key, value);
    }
    for key in &config.environment.remove {
        cmd.env_remove(key);
    }

    // Execute with timeout
    let timeout_duration = std::time::Duration::from_secs(timeout);
    let result = tokio::time::timeout(timeout_duration, cmd.output()).await;

    let output = match result {
        Ok(Ok(output)) => {
            let (stdout, stdout_truncated) =
                truncate_output(&output.stdout, config.limits.max_output_bytes);
            let (stderr, stderr_truncated) =
                truncate_output(&output.stderr, config.limits.max_output_bytes);

            CommandOutput {
                command: format!("(script: {} bytes)", params.script.len()),
                exit_code: output.status.code(),
                stdout,
                stderr,
                timed_out: false,
                truncated: stdout_truncated || stderr_truncated,
            }
        }
        Ok(Err(io_err)) => return Err(exec_error_to_mcp(ExecError::IoError(io_err))),
        Err(_elapsed) => return Err(exec_error_to_mcp(ExecError::Timeout(timeout))),
    };

    json_success(&output)
}
