//! Baseline and regression detection system
//!
//! Manages baseline metrics for benchmark cases and detects performance regressions.

use crate::{BenchmarkResult, BenchmarkSummary, Tier};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Baseline metrics for a single benchmark case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseBaseline {
    /// Case ID
    pub case_id: String,
    /// Model used for baseline
    pub model: String,
    /// Median duration in milliseconds
    pub p50_duration_ms: u64,
    /// 95th percentile duration
    pub p95_duration_ms: u64,
    /// Expected tool call count
    pub expected_tool_count: usize,
    /// Historical success rate (0.0 - 1.0)
    pub success_rate: f64,
    /// Number of runs used to establish baseline
    pub sample_count: usize,
    /// When baseline was established
    pub established_at: DateTime<Utc>,
}

/// Complete baseline for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    /// Model identifier
    pub model: String,
    /// When baseline was created
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Per-case baselines
    pub cases: HashMap<String, CaseBaseline>,
    /// Tier-level summaries
    pub tiers: HashMap<Tier, TierBaseline>,
}

/// Baseline metrics for a tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierBaseline {
    pub tier: Tier,
    pub expected_pass_rate: f64,
    pub p50_duration_ms: u64,
    pub p95_duration_ms: u64,
}

/// Regression severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegressionSeverity {
    /// Minor regression (10-25% slower)
    Minor,
    /// Moderate regression (25-50% slower)
    Moderate,
    /// Severe regression (50%+ slower or failure)
    Severe,
}

/// A detected regression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regression {
    /// Case ID that regressed
    pub case_id: String,
    /// Type of regression
    pub regression_type: RegressionType,
    /// Severity of the regression
    pub severity: RegressionSeverity,
    /// Baseline value
    pub baseline_value: f64,
    /// Current value
    pub current_value: f64,
    /// Percentage change
    pub change_percent: f64,
    /// Human-readable description
    pub description: String,
}

/// Types of regression that can be detected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegressionType {
    /// Duration increased significantly
    DurationRegression,
    /// Success rate dropped
    SuccessRateRegression,
    /// Tool call count changed unexpectedly
    ToolCountRegression,
    /// Previously passing case now fails
    PassToFail,
}

/// Complete regression report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionReport {
    /// Model being compared
    pub model: String,
    /// Baseline model (for comparison)
    pub baseline_model: String,
    /// When report was generated
    pub generated_at: DateTime<Utc>,
    /// All detected regressions
    pub regressions: Vec<Regression>,
    /// Number of cases compared
    pub cases_compared: usize,
    /// Number of cases with regressions
    pub cases_regressed: usize,
    /// Overall assessment
    pub passed: bool,
}

impl RegressionReport {
    /// Check if there are any severe regressions
    pub fn has_severe_regressions(&self) -> bool {
        self.regressions
            .iter()
            .any(|r| r.severity == RegressionSeverity::Severe)
    }

    /// Get regressions by severity
    pub fn by_severity(&self, severity: RegressionSeverity) -> Vec<&Regression> {
        self.regressions
            .iter()
            .filter(|r| r.severity == severity)
            .collect()
    }
}

impl Baseline {
    /// Create a new baseline from benchmark results
    pub fn from_results(model: String, results: &[BenchmarkResult]) -> Self {
        let now = Utc::now();
        let mut cases = HashMap::new();

        // Group results by case_id
        let mut grouped: HashMap<String, Vec<&BenchmarkResult>> = HashMap::new();
        for result in results {
            grouped
                .entry(result.case_id.clone())
                .or_default()
                .push(result);
        }

        // Create baseline for each case
        for (case_id, case_results) in grouped {
            let durations: Vec<u64> = case_results.iter().map(|r| r.duration_ms).collect();
            let success_count = case_results.iter().filter(|r| r.passed).count();
            let tool_counts: Vec<usize> = case_results.iter().map(|r| r.tool_calls.len()).collect();

            let case_baseline = CaseBaseline {
                case_id: case_id.clone(),
                model: model.clone(),
                p50_duration_ms: percentile(&durations, 50.0),
                p95_duration_ms: percentile(&durations, 95.0),
                expected_tool_count: median_usize(&tool_counts),
                success_rate: success_count as f64 / case_results.len() as f64,
                sample_count: case_results.len(),
                established_at: now,
            };

            cases.insert(case_id, case_baseline);
        }

        Self {
            model,
            created_at: now,
            updated_at: now,
            cases,
            tiers: HashMap::new(),
        }
    }

    /// Add tier-level baselines from a summary
    pub fn add_tier_baselines(&mut self, summary: &BenchmarkSummary) {
        for tier_summary in &summary.tiers {
            self.tiers.insert(
                tier_summary.tier,
                TierBaseline {
                    tier: tier_summary.tier,
                    expected_pass_rate: tier_summary.pass_rate,
                    p50_duration_ms: tier_summary.p50_duration_ms,
                    p95_duration_ms: tier_summary.p95_duration_ms,
                },
            );
        }
        self.updated_at = Utc::now();
    }

    /// Compare results against this baseline and generate regression report
    pub fn compare(&self, results: &[BenchmarkResult]) -> RegressionReport {
        let mut regressions = Vec::new();

        for result in results {
            if let Some(baseline) = self.cases.get(&result.case_id) {
                // Check for pass -> fail regression
                if baseline.success_rate > 0.9 && !result.passed {
                    regressions.push(Regression {
                        case_id: result.case_id.clone(),
                        regression_type: RegressionType::PassToFail,
                        severity: RegressionSeverity::Severe,
                        baseline_value: baseline.success_rate * 100.0,
                        current_value: 0.0,
                        change_percent: -100.0,
                        description: format!(
                            "Case '{}' was previously passing ({}% success) but now fails",
                            result.case_id,
                            (baseline.success_rate * 100.0) as u32
                        ),
                    });
                }

                // Check duration regression (only for passing tests)
                if result.passed {
                    let threshold = baseline.p95_duration_ms as f64 * 1.5;
                    if result.duration_ms as f64 > threshold {
                        let change = ((result.duration_ms as f64
                            - baseline.p50_duration_ms as f64)
                            / baseline.p50_duration_ms as f64)
                            * 100.0;
                        let severity = if change > 100.0 {
                            RegressionSeverity::Severe
                        } else if change > 50.0 {
                            RegressionSeverity::Moderate
                        } else {
                            RegressionSeverity::Minor
                        };

                        regressions.push(Regression {
                            case_id: result.case_id.clone(),
                            regression_type: RegressionType::DurationRegression,
                            severity,
                            baseline_value: baseline.p50_duration_ms as f64,
                            current_value: result.duration_ms as f64,
                            change_percent: change,
                            description: format!(
                                "Case '{}' duration increased by {:.1}% ({}ms -> {}ms)",
                                result.case_id,
                                change,
                                baseline.p50_duration_ms,
                                result.duration_ms
                            ),
                        });
                    }

                    // Check tool count regression
                    let tool_diff = (result.tool_calls.len() as i64
                        - baseline.expected_tool_count as i64)
                        .abs();
                    if tool_diff > 2 {
                        regressions.push(Regression {
                            case_id: result.case_id.clone(),
                            regression_type: RegressionType::ToolCountRegression,
                            severity: if tool_diff > 5 {
                                RegressionSeverity::Moderate
                            } else {
                                RegressionSeverity::Minor
                            },
                            baseline_value: baseline.expected_tool_count as f64,
                            current_value: result.tool_calls.len() as f64,
                            change_percent: (tool_diff as f64
                                / baseline.expected_tool_count as f64)
                                * 100.0,
                            description: format!(
                                "Case '{}' tool count changed significantly ({} -> {})",
                                result.case_id,
                                baseline.expected_tool_count,
                                result.tool_calls.len()
                            ),
                        });
                    }
                }
            }
        }

        let cases_regressed = regressions
            .iter()
            .map(|r| &r.case_id)
            .collect::<std::collections::HashSet<_>>()
            .len();

        RegressionReport {
            model: results.first().map(|r| r.model.clone()).unwrap_or_default(),
            baseline_model: self.model.clone(),
            generated_at: Utc::now(),
            passed: !regressions
                .iter()
                .any(|r| r.severity == RegressionSeverity::Severe),
            cases_compared: results.len(),
            cases_regressed,
            regressions,
        }
    }

    /// Load baseline from a file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Save baseline to a file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get default baseline path for a model
    pub fn default_path(model: &str) -> PathBuf {
        let sanitized = model.replace(['/', ':'], "_");
        PathBuf::from("benchmarks/baselines").join(format!("{}.json", sanitized))
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

/// Calculate median from a slice of usize values
fn median_usize(values: &[usize]) -> usize {
    if values.is_empty() {
        return 0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolCallMetric;

    fn make_result(
        case_id: &str,
        passed: bool,
        duration_ms: u64,
        tool_count: usize,
    ) -> BenchmarkResult {
        BenchmarkResult {
            case_id: case_id.to_string(),
            model: "test-model".to_string(),
            passed,
            duration_ms,
            tool_calls: (0..tool_count)
                .map(|i| ToolCallMetric {
                    tool: format!("tool_{}", i),
                    server: "test".to_string(),
                    duration_ms: 10,
                    success: true,
                    error_type: None,
                    timestamp: Utc::now(),
                })
                .collect(),
            iterations: 1,
            error: None,
            output: String::new(),
            timestamp: Utc::now(),
            missing_tools: vec![],
            forbidden_tools_called: vec![],
        }
    }

    #[test]
    fn test_baseline_creation() {
        let results = vec![
            make_result("case1", true, 100, 3),
            make_result("case1", true, 120, 3),
            make_result("case2", true, 200, 5),
        ];

        let baseline = Baseline::from_results("test-model".to_string(), &results);

        assert_eq!(baseline.cases.len(), 2);
        assert!(baseline.cases.contains_key("case1"));
        assert!(baseline.cases.contains_key("case2"));

        let case1 = baseline.cases.get("case1").unwrap();
        assert_eq!(case1.sample_count, 2);
        assert_eq!(case1.success_rate, 1.0);
    }

    #[test]
    fn test_regression_detection() {
        let baseline_results = vec![
            make_result("case1", true, 100, 3),
            make_result("case1", true, 110, 3),
        ];
        let baseline = Baseline::from_results("baseline-model".to_string(), &baseline_results);

        // Test duration regression
        let slow_results = vec![make_result("case1", true, 500, 3)];
        let report = baseline.compare(&slow_results);
        assert!(!report.regressions.is_empty());
        assert!(report
            .regressions
            .iter()
            .any(|r| matches!(r.regression_type, RegressionType::DurationRegression)));

        // Test pass -> fail regression
        let failing_results = vec![make_result("case1", false, 100, 3)];
        let report = baseline.compare(&failing_results);
        assert!(report
            .regressions
            .iter()
            .any(|r| matches!(r.regression_type, RegressionType::PassToFail)));
        assert!(report.has_severe_regressions());
    }

    #[test]
    fn test_percentile() {
        let values = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        // P50 with nearest-rank rounding: idx = 0.5 * 9 = 4.5 -> 5 -> values[5] = 6
        assert_eq!(percentile(&values, 50.0), 6);
        assert_eq!(percentile(&values, 95.0), 10);
    }
}
