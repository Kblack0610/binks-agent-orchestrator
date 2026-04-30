//! Binks (in-process) adapter.
//!
//! Runs the embedded `binks_agent::Agent` against a configured LLM gateway.
//! Ports the Phase-0 logic that previously lived inline in `BenchmarkRunner`.

use super::{HarnessAdapter, HarnessRequest, HarnessRun};
use crate::collector::BenchmarkCollector;
use anyhow::Result;
use async_trait::async_trait;
use binks_agent::agent::{event_channel, Agent};
use binks_agent::config::McpConfig;
use binks_agent::mcp::McpClientPool;
use std::path::PathBuf;
use tokio::time::timeout;

/// Adapter that drives the in-process Binks agent.
pub struct BinksAdapter {
    /// LLM gateway URL (LiteLLM, Ollama, …) — passed straight to `Agent::new`.
    gateway_url: String,
    /// Default model identifier when `HarnessRequest::model` is `None`.
    default_model: String,
    /// Optional MCP config path; falls back to `McpConfig::load()`'s default
    /// search when `None`.
    mcp_config: Option<PathBuf>,
}

impl BinksAdapter {
    pub fn new(
        gateway_url: impl Into<String>,
        default_model: impl Into<String>,
        mcp_config: Option<PathBuf>,
    ) -> Self {
        Self {
            gateway_url: gateway_url.into(),
            default_model: default_model.into(),
            mcp_config,
        }
    }

    pub fn gateway_url(&self) -> &str {
        &self.gateway_url
    }

    pub fn default_model(&self) -> &str {
        &self.default_model
    }
}

#[async_trait]
impl HarnessAdapter for BinksAdapter {
    fn name(&self) -> &str {
        "binks"
    }

    async fn run(&self, req: HarnessRequest) -> Result<HarnessRun> {
        let start = std::time::Instant::now();
        let model = req.model.as_deref().unwrap_or(&self.default_model);

        let mcp_config = if let Some(ref path) = self.mcp_config {
            McpConfig::load_from_path(path)?
        } else {
            McpConfig::load()?.ok_or_else(|| {
                anyhow::anyhow!("No MCP config found. Create .mcp.json or specify --mcp-config")
            })?
        };

        let mcp_pool = McpClientPool::new(mcp_config);
        let (tx, rx) = event_channel();
        let mut agent = Agent::new(&self.gateway_url, model, mcp_pool).with_event_sender(tx);

        let prompt = req.prompt.clone();
        let servers = req.mcp_servers.clone();

        let result = timeout(req.timeout, async {
            let collector_handle =
                tokio::spawn(async move { BenchmarkCollector::collect(rx).await });

            let response = if let Some(ref server_list) = servers {
                let server_refs: Vec<&str> = server_list.iter().map(|s| s.as_str()).collect();
                agent.chat_with_servers(&prompt, &server_refs).await
            } else {
                agent.chat(&prompt).await
            };

            let metrics = collector_handle.await?;
            Ok::<_, anyhow::Error>((response, metrics))
        })
        .await;

        let duration = start.elapsed();

        match result {
            Ok(Ok((agent_result, metrics))) => {
                let (output, error, exit_code) = match agent_result {
                    Ok(text) => (text, metrics.error.clone(), 0),
                    Err(e) => (String::new(), Some(format!("Agent error: {}", e)), -1),
                };
                Ok(HarnessRun {
                    output,
                    stderr: String::new(),
                    tool_calls: metrics.tool_calls,
                    diff: None,
                    files_changed: Vec::new(),
                    tokens_in: None,
                    tokens_out: None,
                    cost_usd: None,
                    duration,
                    exit_code,
                    iterations: metrics.iterations,
                    error,
                })
            }
            Ok(Err(e)) => Ok(HarnessRun {
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
                error: Some(format!("Execution error: {}", e)),
            }),
            Err(_) => Ok(HarnessRun {
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
            }),
        }
    }
}
