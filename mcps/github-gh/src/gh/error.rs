//! Error types for gh CLI operations
//!
//! This module defines error types that can occur when executing gh commands,
//! including command failures, parsing errors, and authentication issues.

use thiserror::Error;

/// Errors that can occur when executing gh CLI commands
#[derive(Error, Debug)]
pub enum GhError {
    /// The gh command failed with a non-zero exit code
    #[error("gh command failed (exit code {code}): {stderr}")]
    CommandFailed {
        /// Exit code from the gh process
        code: i32,
        /// Standard error output from gh
        stderr: String,
    },

    /// Failed to spawn the gh process
    #[error("failed to spawn gh process: {0}")]
    SpawnError(#[from] std::io::Error),

    /// Failed to parse JSON output from gh
    #[error("failed to parse gh JSON output: {0}")]
    ParseError(#[from] serde_json::Error),

    /// gh CLI is not installed or not in PATH
    #[error("gh CLI not found - ensure gh is installed and in PATH")]
    NotFound,

    /// gh CLI is not authenticated
    #[error("gh CLI not authenticated - run 'gh auth login' first")]
    NotAuthenticated,
}

/// Result type alias for gh operations
pub type GhResult<T> = Result<T, GhError>;
