//! Error types for linear CLI operations

use thiserror::Error;

/// Errors that can occur when executing linear CLI commands
#[derive(Error, Debug)]
pub enum LinearError {
    /// The linear command failed with a non-zero exit code
    #[error("linear command failed (exit code {code}): {stderr}")]
    CommandFailed {
        /// Exit code from the linear process
        code: i32,
        /// Standard error output
        stderr: String,
    },

    /// Failed to spawn the linear process
    #[error("failed to spawn linear process: {0}")]
    SpawnError(#[from] std::io::Error),

    /// Failed to parse JSON output
    #[error("failed to parse linear JSON output: {0}")]
    ParseError(#[from] serde_json::Error),

    /// linear CLI is not installed or not in PATH
    #[error("linear CLI not found - install with: brew install schpet/tap/linear")]
    NotFound,
}

/// Result type alias for linear operations
pub type LinearResult<T> = Result<T, LinearError>;
