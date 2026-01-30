//! Error types for doctl CLI operations
//!
//! Provides a unified error type for all doctl command execution failures.

use thiserror::Error;

/// Errors that can occur when executing doctl commands
#[derive(Error, Debug)]
pub enum DoctlError {
    /// doctl command exited with a non-zero status code
    #[error("doctl command failed (exit code {code}): {stderr}")]
    CommandFailed { code: i32, stderr: String },

    /// Failed to spawn the doctl process
    #[error("failed to spawn doctl process: {0}")]
    SpawnError(#[from] std::io::Error),

    /// Failed to parse JSON output from doctl
    #[error("failed to parse doctl JSON output: {0}")]
    ParseError(#[from] serde_json::Error),

    /// doctl CLI binary not found in PATH
    #[error("doctl CLI not found - ensure doctl is installed and in PATH")]
    NotFound,

    /// doctl CLI not authenticated
    #[error("doctl CLI not authenticated - run 'doctl auth init' first")]
    NotAuthenticated,
}

/// Result type alias for doctl operations
pub type DoctlResult<T> = Result<T, DoctlError>;
