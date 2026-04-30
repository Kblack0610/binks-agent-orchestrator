//! Claude Code adapter.
//!
//! Shells out to `claude -p --output-format json` in the requested workspace
//! and parses the final JSON result for output text, token counts, and cost.
//!
//! Tool-call metrics are NOT yet captured — that requires
//! `--output-format stream-json` parsing, which is a follow-up. Cases that
//! validate `expected_tools` against `mcp__*` names won't fire under this
//! adapter; that's expected.
//!
//! Sandbox: runs with `--bare --permission-mode bypassPermissions` so the
//! benchmark is reproducible (no user CLAUDE.md / hooks / plugins) and fully
//! automated. The plan-locked v1 sandbox is "trust the worktree boundary."

use super::{HarnessAdapter, HarnessRequest, HarnessRun};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Deserialize)]
struct ClaudeJsonResult {
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    is_error: bool,
    #[serde(default)]
    total_cost_usd: Option<f64>,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
    #[serde(default)]
    error: Option<serde_json::Value>,
    #[serde(default)]
    num_turns: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
}

/// Adapter that drives the `claude` CLI in non-interactive mode.
pub struct ClaudeCodeAdapter {
    default_model: String,
    binary: String,
}

impl ClaudeCodeAdapter {
    /// `default_model` is what `claude --model` receives when
    /// [`HarnessRequest::model`] is `None`. Use a Claude alias (`sonnet`,
    /// `opus`, `haiku`) or a full model id (`claude-sonnet-4-6`).
    pub fn new(default_model: impl Into<String>) -> Self {
        Self {
            default_model: default_model.into(),
            binary: "claude".to_string(),
        }
    }

    /// Override the binary path (handy for testing or unusual installs).
    pub fn with_binary(mut self, binary: impl Into<String>) -> Self {
        self.binary = binary.into();
        self
    }

    pub fn default_model(&self) -> &str {
        &self.default_model
    }
}

#[async_trait]
impl HarnessAdapter for ClaudeCodeAdapter {
    fn name(&self) -> &str {
        "claude-code"
    }

    async fn run(&self, req: HarnessRequest) -> Result<HarnessRun> {
        let start = std::time::Instant::now();
        let model = req.model.as_deref().unwrap_or(&self.default_model);

        let mut cmd = Command::new(&self.binary);
        cmd.arg("-p")
            .arg(&req.prompt)
            .arg("--output-format")
            .arg("json")
            .arg("--model")
            .arg(model)
            .arg("--bare")
            .arg("--permission-mode")
            .arg("bypassPermissions")
            .current_dir(&req.workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(allowlist) = &req.allowed_tools {
            if !allowlist.is_empty() {
                cmd.arg("--allowedTools").args(allowlist);
            }
        }
        for (k, v) in &req.env {
            cmd.env(k, v);
        }

        let child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn `{}` (is Claude Code installed?)", self.binary))?;

        let output_result = timeout(req.timeout, child.wait_with_output()).await;
        let duration = start.elapsed();

        let output = match output_result {
            Err(_) => {
                return Ok(timeout_run(&req, duration));
            }
            Ok(Err(e)) => {
                return Ok(error_run(duration, format!("Process error: {}", e)));
            }
            Ok(Ok(o)) => o,
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        let parsed: Option<ClaudeJsonResult> = serde_json::from_str(stdout.trim()).ok();

        let (text, error, tokens_in, tokens_out, cost, iterations) = match parsed {
            Some(r) => {
                let err_msg = if r.is_error {
                    Some(
                        r.error
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "claude reported is_error=true".to_string()),
                    )
                } else {
                    None
                };
                let (ti, to) = r
                    .usage
                    .as_ref()
                    .map(|u| (u.input_tokens, u.output_tokens))
                    .unwrap_or((None, None));
                (
                    r.result.unwrap_or_default(),
                    err_msg,
                    ti,
                    to,
                    r.total_cost_usd,
                    r.num_turns.unwrap_or(1),
                )
            }
            None => (
                stdout.clone(),
                if exit_code != 0 {
                    Some(format!(
                        "claude exited {} and stdout was not valid JSON",
                        exit_code
                    ))
                } else {
                    None
                },
                None,
                None,
                None,
                1,
            ),
        };

        Ok(HarnessRun {
            output: text,
            stderr,
            tool_calls: Vec::new(),
            diff: None,
            files_changed: Vec::new(),
            tokens_in,
            tokens_out,
            cost_usd: cost,
            duration,
            exit_code,
            iterations,
            error,
        })
    }
}

fn timeout_run(req: &HarnessRequest, duration: std::time::Duration) -> HarnessRun {
    HarnessRun {
        output: String::new(),
        stderr: String::new(),
        tool_calls: Vec::new(),
        diff: None,
        files_changed: Vec::new(),
        tokens_in: None,
        tokens_out: None,
        cost_usd: None,
        duration,
        exit_code: -1,
        iterations: 0,
        error: Some(format!("Timeout after {:?}", req.timeout)),
    }
}

fn error_run(duration: std::time::Duration, msg: String) -> HarnessRun {
    HarnessRun {
        output: String::new(),
        stderr: String::new(),
        tool_calls: Vec::new(),
        diff: None,
        files_changed: Vec::new(),
        tokens_in: None,
        tokens_out: None,
        cost_usd: None,
        duration,
        exit_code: -1,
        iterations: 0,
        error: Some(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_claude_json_minimal() {
        let json = r#"{"type":"result","result":"hello","is_error":false,"total_cost_usd":0.0042,"usage":{"input_tokens":120,"output_tokens":35},"num_turns":2}"#;
        let parsed: ClaudeJsonResult = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.result.as_deref(), Some("hello"));
        assert!(!parsed.is_error);
        assert_eq!(parsed.total_cost_usd, Some(0.0042));
        assert_eq!(parsed.usage.as_ref().unwrap().input_tokens, Some(120));
        assert_eq!(parsed.usage.as_ref().unwrap().output_tokens, Some(35));
        assert_eq!(parsed.num_turns, Some(2));
    }

    #[test]
    fn parse_claude_json_error() {
        let json = r#"{"type":"result","is_error":true,"error":"rate_limited"}"#;
        let parsed: ClaudeJsonResult = serde_json::from_str(json).unwrap();
        assert!(parsed.is_error);
        assert!(parsed.error.is_some());
    }

    #[test]
    fn parse_claude_json_extra_fields_ignored() {
        let json = r#"{"type":"result","result":"ok","is_error":false,"session_id":"abc","duration_ms":1234,"some_future_field":42}"#;
        let parsed: ClaudeJsonResult = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.result.as_deref(), Some("ok"));
    }
}
