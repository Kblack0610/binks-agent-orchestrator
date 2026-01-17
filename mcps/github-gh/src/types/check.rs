//! Check run type definitions
//!
//! Structs representing GitHub check run data as returned by gh CLI.

use serde::{Deserialize, Serialize};

/// Represents a GitHub check run (CI/CD job)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRun {
    /// Check run name
    pub name: String,

    /// Status (queued, in_progress, completed)
    pub status: String,

    /// Conclusion (success, failure, neutral, cancelled, timed_out, action_required, skipped)
    #[serde(default)]
    pub conclusion: Option<String>,

    /// Started timestamp (ISO 8601)
    #[serde(default)]
    pub started_at: Option<String>,

    /// Completed timestamp (ISO 8601)
    #[serde(default)]
    pub completed_at: Option<String>,

    /// Details URL
    #[serde(default)]
    pub details_url: Option<String>,
}

/// Status check rollup for a commit/PR
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusCheckRollup {
    /// Overall state (PENDING, SUCCESS, FAILURE, ERROR)
    #[serde(default)]
    pub state: Option<String>,

    /// Individual check contexts
    #[serde(default)]
    pub contexts: Vec<CheckContext>,
}

/// Individual check context in a rollup
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckContext {
    /// Check name
    #[serde(default)]
    pub name: Option<String>,

    /// Context name (for status checks)
    #[serde(default)]
    pub context: Option<String>,

    /// State/status
    #[serde(default)]
    pub state: Option<String>,

    /// Conclusion
    #[serde(default)]
    pub conclusion: Option<String>,

    /// Target URL
    #[serde(default)]
    pub target_url: Option<String>,
}

impl CheckRun {
    /// Returns the JSON fields to request from gh CLI
    pub fn list_fields() -> &'static [&'static str] {
        &[
            "name",
            "status",
            "conclusion",
            "startedAt",
            "completedAt",
            "detailsUrl",
        ]
    }
}
