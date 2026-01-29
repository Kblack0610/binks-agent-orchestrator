//! Benchmark runner
//!
//! Executes benchmark cases against the Binks agent and collects results.

use crate::{
    collector::{BenchmarkCollector, CollectedMetrics},
    BenchmarkCase, BenchmarkResult, BenchmarkSummary, Tier, TierSummary,
};
use agent::agent::{event_channel, Agent};
use agent::config::McpConfig;
use agent::mcp::McpClientPool;
use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;
use tokio::time::timeout;

/// Configuration for the benchmark runner
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Ollama base URL
    pub ollama_url: String,
    /// Default model to use
    pub model: String,
    /// MCP config path (optional, uses default if None)
    pub mcp_config: Option<String>,
    /// Whether to print verbose output
    pub verbose: bool,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            model: "llama3.1:8b".to_string(),
            mcp_config: None,
            verbose: false,
        }
    }
}

/// Benchmark runner
pub struct BenchmarkRunner {
    config: RunnerConfig,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner
    pub fn new(config: RunnerConfig) -> Self {
        Self { config }
    }

    /// Create with just URL and model
    pub fn with_model(ollama_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(RunnerConfig {
            ollama_url: ollama_url.into(),
            model: model.into(),
            ..Default::default()
        })
    }

    /// Run a single benchmark case
    pub async fn run_case(&self, case: &BenchmarkCase) -> Result<BenchmarkResult> {
        let start = std::time::Instant::now();
        let timestamp = Utc::now();

        tracing::info!(case_id = %case.id, tier = %case.tier, "Running benchmark case");

        // Load MCP config
        let mcp_config = if let Some(ref path) = self.config.mcp_config {
            McpConfig::load_from_path(&PathBuf::from(path))?
        } else {
            McpConfig::load()?.ok_or_else(|| {
                anyhow::anyhow!("No MCP config found. Create .mcp.json or specify --mcp-config")
            })?
        };

        // Create MCP pool and agent with event channel
        let mcp_pool = McpClientPool::new(mcp_config);
        let (tx, rx) = event_channel();
        let mut agent =
            Agent::new(&self.config.ollama_url, &self.config.model, mcp_pool).with_event_sender(tx);

        // Clone servers for use in async block
        let servers = case.servers.clone();
        let prompt = case.prompt.clone();

        // Run the benchmark with timeout
        let result = timeout(case.timeout, async {
            // Spawn collector task
            let collector_handle =
                tokio::spawn(async move { BenchmarkCollector::collect(rx).await });

            // Run agent (filter by servers if specified)
            let response = if let Some(ref server_list) = servers {
                let server_refs: Vec<&str> = server_list.iter().map(|s| s.as_str()).collect();
                agent.chat_with_servers(&prompt, &server_refs).await
            } else {
                agent.chat(&prompt).await
            };

            // Wait for collector
            let metrics = collector_handle.await?;

            Ok::<(Result<String, _>, CollectedMetrics), anyhow::Error>((response, metrics))
        })
        .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok((agent_result, metrics))) => {
                let output = match agent_result {
                    Ok(response) => response,
                    Err(e) => {
                        return Ok(self.create_result(
                            case,
                            false,
                            duration_ms,
                            metrics.tool_calls,
                            Some(format!("Agent error: {}", e)),
                            String::new(),
                            timestamp,
                        ));
                    }
                };

                // Validate results
                let tools_called: Vec<&str> =
                    metrics.tool_calls.iter().map(|t| t.tool.as_str()).collect();

                // Check for missing expected tools
                let missing_tools: Vec<String> = case
                    .expected_tools
                    .iter()
                    .filter(|t| !tools_called.contains(&t.as_str()))
                    .cloned()
                    .collect();

                // Check for forbidden tools that were called
                let forbidden_called: Vec<String> = case
                    .forbidden_tools
                    .iter()
                    .filter(|t| tools_called.contains(&t.as_str()))
                    .cloned()
                    .collect();

                // Check success criteria
                let mut result = self.create_result(
                    case,
                    false, // Will set below
                    duration_ms,
                    metrics.tool_calls,
                    metrics.error,
                    output,
                    timestamp,
                );

                result.missing_tools = missing_tools;
                result.forbidden_tools_called = forbidden_called;

                // Determine pass/fail
                result.passed = case.success_criteria.is_satisfied(&result)
                    && result.missing_tools.is_empty()
                    && result.forbidden_tools_called.is_empty();

                if result.passed {
                    tracing::info!(
                        case_id = %case.id,
                        duration_ms = duration_ms,
                        "Benchmark PASSED"
                    );
                } else {
                    tracing::warn!(
                        case_id = %case.id,
                        duration_ms = duration_ms,
                        missing_tools = ?result.missing_tools,
                        forbidden_called = ?result.forbidden_tools_called,
                        "Benchmark FAILED"
                    );
                }

                Ok(result)
            }
            Ok(Err(e)) => Ok(self.create_result(
                case,
                false,
                duration_ms,
                vec![],
                Some(format!("Execution error: {}", e)),
                String::new(),
                timestamp,
            )),
            Err(_) => Ok(self.create_result(
                case,
                false,
                duration_ms,
                vec![],
                Some(format!("Timeout after {:?}", case.timeout)),
                String::new(),
                timestamp,
            )),
        }
    }

    /// Run all benchmark cases
    pub async fn run_all(&self, cases: &[BenchmarkCase]) -> Result<Vec<BenchmarkResult>> {
        let mut results = Vec::with_capacity(cases.len());

        for case in cases {
            let result = self.run_case(case).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Run benchmark cases for a specific tier
    pub async fn run_tier(
        &self,
        cases: &[BenchmarkCase],
        tier: Tier,
    ) -> Result<Vec<BenchmarkResult>> {
        let tier_cases: Vec<_> = cases.iter().filter(|c| c.tier == tier).collect();
        let mut results = Vec::with_capacity(tier_cases.len());

        for case in tier_cases {
            let result = self.run_case(case).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Create a benchmark result
    #[allow(clippy::too_many_arguments)]
    fn create_result(
        &self,
        case: &BenchmarkCase,
        passed: bool,
        duration_ms: u64,
        tool_calls: Vec<crate::ToolCallMetric>,
        error: Option<String>,
        output: String,
        timestamp: chrono::DateTime<Utc>,
    ) -> BenchmarkResult {
        BenchmarkResult {
            case_id: case.id.clone(),
            model: self.config.model.clone(),
            passed,
            duration_ms,
            tool_calls,
            iterations: 1,
            error,
            output,
            timestamp,
            missing_tools: vec![],
            forbidden_tools_called: vec![],
        }
    }

    /// Generate a summary from results
    pub fn summarize(
        &self,
        results: &[BenchmarkResult],
        cases: &[BenchmarkCase],
    ) -> BenchmarkSummary {
        let mut tier_results: std::collections::HashMap<Tier, Vec<&BenchmarkResult>> =
            std::collections::HashMap::new();

        for result in results {
            if let Some(case) = cases.iter().find(|c| c.id == result.case_id) {
                tier_results.entry(case.tier).or_default().push(result);
            }
        }

        let tiers: Vec<TierSummary> = [Tier::Tier1, Tier::Tier2, Tier::Tier3, Tier::Platform]
            .iter()
            .filter_map(|tier| {
                tier_results.get(tier).map(|results| {
                    let passed = results.iter().filter(|r| r.passed).count();
                    let total = results.len();
                    let durations: Vec<u64> = results.iter().map(|r| r.duration_ms).collect();

                    TierSummary {
                        tier: *tier,
                        total,
                        passed,
                        failed: total - passed,
                        pass_rate: if total > 0 {
                            (passed as f64 / total as f64) * 100.0
                        } else {
                            0.0
                        },
                        avg_duration_ms: if total > 0 {
                            durations.iter().sum::<u64>() / total as u64
                        } else {
                            0
                        },
                        p50_duration_ms: percentile(&durations, 50.0),
                        p95_duration_ms: percentile(&durations, 95.0),
                    }
                })
            })
            .collect();

        let total_passed = results.iter().filter(|r| r.passed).count();
        let total_cases = results.len();

        BenchmarkSummary {
            model: self.config.model.clone(),
            timestamp: Utc::now(),
            tiers,
            total_cases,
            total_passed,
            total_failed: total_cases - total_passed,
            overall_pass_rate: if total_cases > 0 {
                (total_passed as f64 / total_cases as f64) * 100.0
            } else {
                0.0
            },
            total_duration_ms: results.iter().map(|r| r.duration_ms).sum(),
        }
    }
}

/// Calculate percentile from a slice of values
fn percentile(values: &[u64], p: f64) -> u64 {
    if values.is_empty() {
        return 0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_unstable();

    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile() {
        let values = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        // P50 with nearest-rank rounding: idx = 0.5 * 9 = 4.5 -> 5 -> values[5] = 6
        assert_eq!(percentile(&values, 50.0), 6);
        assert_eq!(percentile(&values, 95.0), 10);
        assert_eq!(percentile(&values, 0.0), 1);
        assert_eq!(percentile(&values, 100.0), 10);
    }

    #[test]
    fn test_percentile_empty() {
        let values: Vec<u64> = vec![];
        assert_eq!(percentile(&values, 50.0), 0);
    }
}
