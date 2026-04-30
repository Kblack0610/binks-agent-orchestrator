//! Benchmark runner
//!
//! Dispatches a [`BenchmarkCase`] through a [`HarnessAdapter`], collects the
//! resulting [`HarnessRun`], and applies the case's [`crate::SuccessCriteria`]
//! / expected-tool / forbidden-tool policy to produce a [`BenchmarkResult`].
//!
//! The runner is harness-agnostic: it knows nothing about Ollama, Claude Code,
//! opencode, etc. — only the adapter does.

use crate::adapters::{BinksAdapter, HarnessAdapter, HarnessRequest, HarnessRun};
use crate::{BenchmarkCase, BenchmarkResult, BenchmarkSummary, Tier, TierSummary};
use anyhow::Result;
use chrono::Utc;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Configuration for the benchmark runner.
///
/// Constructs a default [`BinksAdapter`] for back-compat with the existing
/// `--gateway-url` / `--model` / `--mcp-config` CLI flags. Phase 3 will allow
/// callers to inject any adapter via [`BenchmarkRunner::with_adapter`].
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// LLM gateway URL (LiteLLM, Ollama, …).
    pub gateway_url: String,
    /// Default model identifier.
    pub model: String,
    /// MCP config path (uses default if `None`).
    pub mcp_config: Option<String>,
    /// Whether to print verbose output.
    pub verbose: bool,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            gateway_url: "http://localhost:11434".to_string(),
            model: "llama3.1:8b".to_string(),
            mcp_config: None,
            verbose: false,
        }
    }
}

/// Benchmark runner.
pub struct BenchmarkRunner {
    adapter: Arc<dyn HarnessAdapter>,
    /// Model label recorded on every result. Defaults to the adapter's
    /// configured model; `with_adapter` callers can override via
    /// [`BenchmarkRunner::with_model_label`].
    model_label: String,
}

impl BenchmarkRunner {
    /// Build a runner backed by the default in-process [`BinksAdapter`].
    pub fn new(config: RunnerConfig) -> Self {
        let adapter = BinksAdapter::new(
            config.gateway_url,
            config.model.clone(),
            config.mcp_config.map(PathBuf::from),
        );
        Self {
            adapter: Arc::new(adapter),
            model_label: config.model,
        }
    }

    /// Build a runner from a URL/model pair (Binks adapter, no MCP override).
    pub fn with_model(gateway_url: impl Into<String>, model: impl Into<String>) -> Self {
        let model_str = model.into();
        Self::new(RunnerConfig {
            gateway_url: gateway_url.into(),
            model: model_str,
            mcp_config: None,
            verbose: false,
        })
    }

    /// Build a runner around an arbitrary adapter. The `model_label` is what
    /// [`BenchmarkResult::model`] gets stamped with.
    pub fn with_adapter(adapter: Arc<dyn HarnessAdapter>, model_label: impl Into<String>) -> Self {
        Self {
            adapter,
            model_label: model_label.into(),
        }
    }

    /// Override the model label recorded on results without changing the adapter.
    pub fn with_model_label(mut self, label: impl Into<String>) -> Self {
        self.model_label = label.into();
        self
    }

    /// Adapter name (`"binks"`, `"claude-code"`, …) — useful for reporters.
    pub fn harness_name(&self) -> &str {
        self.adapter.name()
    }

    /// Run a single benchmark case.
    pub async fn run_case(&self, case: &BenchmarkCase) -> Result<BenchmarkResult> {
        let timestamp = Utc::now();
        tracing::info!(case_id = %case.id, tier = %case.tier, "Running benchmark case");

        let request = HarnessRequest {
            prompt: case.prompt.clone(),
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            model: None,
            mcp_servers: case.servers.clone(),
            allowed_tools: None,
            timeout: case.timeout,
            env: BTreeMap::new(),
        };

        let run = self.adapter.run(request).await?;
        let result = self.evaluate(case, run, timestamp);

        if result.passed {
            tracing::info!(
                case_id = %case.id,
                duration_ms = result.duration_ms,
                "Benchmark PASSED"
            );
        } else {
            tracing::warn!(
                case_id = %case.id,
                duration_ms = result.duration_ms,
                missing_tools = ?result.missing_tools,
                forbidden_called = ?result.forbidden_tools_called,
                "Benchmark FAILED"
            );
        }

        Ok(result)
    }

    /// Run all benchmark cases.
    pub async fn run_all(&self, cases: &[BenchmarkCase]) -> Result<Vec<BenchmarkResult>> {
        let mut results = Vec::with_capacity(cases.len());
        for case in cases {
            results.push(self.run_case(case).await?);
        }
        Ok(results)
    }

    /// Run benchmark cases for a specific tier.
    pub async fn run_tier(
        &self,
        cases: &[BenchmarkCase],
        tier: Tier,
    ) -> Result<Vec<BenchmarkResult>> {
        let tier_cases: Vec<_> = cases.iter().filter(|c| c.tier == tier).collect();
        let mut results = Vec::with_capacity(tier_cases.len());
        for case in tier_cases {
            results.push(self.run_case(case).await?);
        }
        Ok(results)
    }

    /// Apply the case's success / expected-tool / forbidden-tool policy to a
    /// raw [`HarnessRun`] and produce a scored [`BenchmarkResult`].
    fn evaluate(
        &self,
        case: &BenchmarkCase,
        run: HarnessRun,
        timestamp: chrono::DateTime<Utc>,
    ) -> BenchmarkResult {
        let duration_ms = run.duration.as_millis() as u64;

        // Bail early on adapter-reported terminal errors; nothing to score.
        if let Some(err) = &run.error {
            if run.output.is_empty() {
                return BenchmarkResult {
                    case_id: case.id.clone(),
                    model: self.model_label.clone(),
                    passed: false,
                    duration_ms,
                    tool_calls: run.tool_calls,
                    iterations: run.iterations.max(1),
                    error: Some(err.clone()),
                    output: String::new(),
                    timestamp,
                    missing_tools: Vec::new(),
                    forbidden_tools_called: Vec::new(),
                };
            }
        }

        let tools_called: Vec<&str> = run.tool_calls.iter().map(|t| t.tool.as_str()).collect();

        let missing_tools: Vec<String> = case
            .expected_tools
            .iter()
            .filter(|t| !tools_called.contains(&t.as_str()))
            .cloned()
            .collect();

        let forbidden_called: Vec<String> = case
            .forbidden_tools
            .iter()
            .filter(|t| tools_called.contains(&t.as_str()))
            .cloned()
            .collect();

        let mut result = BenchmarkResult {
            case_id: case.id.clone(),
            model: self.model_label.clone(),
            passed: false,
            duration_ms,
            tool_calls: run.tool_calls,
            iterations: run.iterations.max(1),
            error: run.error,
            output: run.output,
            timestamp,
            missing_tools,
            forbidden_tools_called: forbidden_called,
        };

        result.passed = case.success_criteria.is_satisfied(&result)
            && result.missing_tools.is_empty()
            && result.forbidden_tools_called.is_empty();

        result
    }

    /// Generate a summary from results.
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
            model: self.model_label.clone(),
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
