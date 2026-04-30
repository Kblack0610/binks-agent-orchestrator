//! Harness adapters
//!
//! A `HarnessAdapter` wraps any coding harness (Binks, Claude Code, opencode,
//! OpenClaw, aider, …) behind a uniform interface so the benchmark runner stays
//! harness-agnostic. Phase 1 ships only `BinksAdapter` (the existing in-process
//! agent path); subsequent phases add adapters that shell out to external CLIs.
//!
//! Contract: an adapter receives a [`HarnessRequest`] (prompt, workspace,
//! model, optional tool/server filters, timeout) and returns a [`HarnessRun`]
//! describing what happened (output, tool calls, optional diff, token/cost
//! telemetry). Pass/fail evaluation against [`crate::SuccessCriteria`] is the
//! runner's job, not the adapter's.

use crate::ToolCallMetric;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

pub mod binks;

pub use binks::BinksAdapter;

/// One unit of work for a harness.
#[derive(Debug, Clone)]
pub struct HarnessRequest {
    /// User-facing prompt to drive the harness with.
    pub prompt: String,
    /// Working directory the harness should operate in (CWD for shelled
    /// harnesses; advisory for in-process ones like Binks).
    pub workspace: PathBuf,
    /// Model identifier. For LiteLLM-routed adapters this is the slot name
    /// (`code`, `reasoning`, …); for adapters that talk to a provider directly
    /// it can be a `provider/model` string. `None` means use the adapter's
    /// configured default.
    pub model: Option<String>,
    /// Restrict the harness to this MCP server set, if it supports filtering.
    /// Mirrors the existing `BenchmarkCase::servers` field for the Binks path.
    pub mcp_servers: Option<Vec<String>>,
    /// Restrict the harness to this tool allowlist, if it supports it
    /// (e.g. Claude Code's `--allowedTools`). Ignored by adapters that don't.
    pub allowed_tools: Option<Vec<String>>,
    /// Wall-clock timeout for the entire run.
    pub timeout: Duration,
    /// Extra environment variables for shelled adapters. Ignored by
    /// in-process adapters.
    pub env: BTreeMap<String, String>,
}

/// Result of one harness run. Fail/pass scoring happens at a higher layer.
#[derive(Debug, Clone)]
pub struct HarnessRun {
    /// Final assistant output (stdout for shelled harnesses, response text
    /// for in-process ones).
    pub output: String,
    /// Anything the harness wrote to stderr; empty for in-process adapters.
    pub stderr: String,
    /// Tool calls observed during the run, when the harness exposes them.
    pub tool_calls: Vec<ToolCallMetric>,
    /// `git diff` against the base ref, when the harness modified files.
    /// `None` for conversational runs that don't touch a repo.
    pub diff: Option<String>,
    /// Files modified relative to `workspace`. Empty when no diff.
    pub files_changed: Vec<PathBuf>,
    /// Prompt tokens consumed, if reported.
    pub tokens_in: Option<u64>,
    /// Completion tokens produced, if reported.
    pub tokens_out: Option<u64>,
    /// Cost in USD if the harness or gateway reports it.
    pub cost_usd: Option<f64>,
    /// Wall-clock duration of the run.
    pub duration: Duration,
    /// Process exit code for shelled harnesses; `0` for successful
    /// in-process runs and `-1` for in-process errors.
    pub exit_code: i32,
    /// Number of agent iterations, when the harness reports it. Preserved
    /// for compatibility with the existing tier summary.
    pub iterations: usize,
    /// Terminal error message if the run failed before producing output.
    pub error: Option<String>,
}

/// A pluggable coding harness.
#[async_trait]
pub trait HarnessAdapter: Send + Sync {
    /// Stable identifier (`"binks"`, `"claude-code"`, `"opencode"`, …).
    fn name(&self) -> &str;

    /// Execute one run.
    async fn run(&self, req: HarnessRequest) -> Result<HarnessRun>;
}
