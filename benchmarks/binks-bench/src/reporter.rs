//! Benchmark result reporting
//!
//! Generates formatted output in various formats (Markdown, JSON, CSV).

use crate::{BenchmarkResult, BenchmarkSummary, RegressionReport};
use anyhow::Result;
use std::io::Write;

/// Output format for benchmark reports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable Markdown
    Markdown,
    /// Machine-readable JSON
    Json,
    /// Spreadsheet-compatible CSV
    Csv,
    /// Compact terminal output
    Terminal,
}

impl std::str::FromStr for OutputFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "md" | "markdown" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            "terminal" | "term" | "console" => Ok(Self::Terminal),
            _ => Err(anyhow::anyhow!("Unknown format: {}", s)),
        }
    }
}

/// Benchmark report generator
pub struct Reporter {
    format: OutputFormat,
}

impl Reporter {
    /// Create a new reporter with the specified format
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    /// Generate a summary report
    pub fn summary(&self, summary: &BenchmarkSummary) -> String {
        match self.format {
            OutputFormat::Markdown => self.summary_markdown(summary),
            OutputFormat::Json => self.summary_json(summary),
            OutputFormat::Csv => self.summary_csv(summary),
            OutputFormat::Terminal => self.summary_terminal(summary),
        }
    }

    /// Generate detailed results report
    pub fn results(&self, results: &[BenchmarkResult]) -> String {
        match self.format {
            OutputFormat::Markdown => self.results_markdown(results),
            OutputFormat::Json => self.results_json(results),
            OutputFormat::Csv => self.results_csv(results),
            OutputFormat::Terminal => self.results_terminal(results),
        }
    }

    /// Generate regression report
    pub fn regression(&self, report: &RegressionReport) -> String {
        match self.format {
            OutputFormat::Markdown => self.regression_markdown(report),
            OutputFormat::Json => self.regression_json(report),
            OutputFormat::Csv => self.regression_csv(report),
            OutputFormat::Terminal => self.regression_terminal(report),
        }
    }

    /// Write report to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W, content: &str) -> Result<()> {
        write!(writer, "{}", content)?;
        Ok(())
    }

    // === Markdown formatters ===

    fn summary_markdown(&self, summary: &BenchmarkSummary) -> String {
        let mut output = String::new();

        output.push_str("# Benchmark Summary\n\n");
        output.push_str(&format!("**Model:** {}\n", summary.model));
        output.push_str(&format!(
            "**Date:** {}\n",
            summary.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        output.push_str(&format!(
            "**Total Duration:** {}ms\n\n",
            summary.total_duration_ms
        ));

        output.push_str("## Overall Results\n\n");
        output.push_str(&format!("- **Total Cases:** {}\n", summary.total_cases));
        output.push_str(&format!("- **Passed:** {}\n", summary.total_passed));
        output.push_str(&format!("- **Failed:** {}\n", summary.total_failed));
        output.push_str(&format!(
            "- **Pass Rate:** {:.1}%\n\n",
            summary.overall_pass_rate
        ));

        output.push_str("## Results by Tier\n\n");
        output.push_str(
            "| Tier | Total | Passed | Failed | Pass Rate | Avg Duration | P50 | P95 |\n",
        );
        output.push_str(
            "|------|-------|--------|--------|-----------|--------------|-----|-----|\n",
        );

        for tier in &summary.tiers {
            output.push_str(&format!(
                "| {} | {} | {} | {} | {:.1}% | {}ms | {}ms | {}ms |\n",
                tier.tier.name(),
                tier.total,
                tier.passed,
                tier.failed,
                tier.pass_rate,
                tier.avg_duration_ms,
                tier.p50_duration_ms,
                tier.p95_duration_ms
            ));
        }

        output
    }

    fn results_markdown(&self, results: &[BenchmarkResult]) -> String {
        let mut output = String::new();

        output.push_str("# Detailed Benchmark Results\n\n");

        for result in results {
            let status = if result.passed {
                "✅ PASS"
            } else {
                "❌ FAIL"
            };
            output.push_str(&format!("## {} - {}\n\n", result.case_id, status));
            output.push_str(&format!("- **Duration:** {}ms\n", result.duration_ms));
            output.push_str(&format!("- **Tool Calls:** {}\n", result.tool_calls.len()));

            if !result.tool_calls.is_empty() {
                output.push_str("\n**Tools Used:**\n");
                for tc in &result.tool_calls {
                    let status = if tc.success { "✓" } else { "✗" };
                    output.push_str(&format!(
                        "- {} `{}` ({}ms)\n",
                        status, tc.tool, tc.duration_ms
                    ));
                }
            }

            if !result.missing_tools.is_empty() {
                output.push_str(&format!(
                    "\n**Missing Tools:** {}\n",
                    result.missing_tools.join(", ")
                ));
            }

            if !result.forbidden_tools_called.is_empty() {
                output.push_str(&format!(
                    "\n**Forbidden Tools Called:** {}\n",
                    result.forbidden_tools_called.join(", ")
                ));
            }

            if let Some(error) = &result.error {
                output.push_str(&format!("\n**Error:** {}\n", error));
            }

            output.push_str("\n---\n\n");
        }

        output
    }

    fn regression_markdown(&self, report: &RegressionReport) -> String {
        let mut output = String::new();

        let status = if report.passed {
            "✅ PASSED"
        } else {
            "❌ FAILED"
        };
        output.push_str(&format!("# Regression Report - {}\n\n", status));
        output.push_str(&format!(
            "**Model:** {} (vs baseline: {})\n",
            report.model, report.baseline_model
        ));
        output.push_str(&format!(
            "**Date:** {}\n",
            report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        output.push_str(&format!("**Cases Compared:** {}\n", report.cases_compared));
        output.push_str(&format!(
            "**Cases with Regressions:** {}\n\n",
            report.cases_regressed
        ));

        if report.regressions.is_empty() {
            output.push_str("No regressions detected.\n");
        } else {
            output.push_str("## Detected Regressions\n\n");
            output.push_str("| Case | Type | Severity | Baseline | Current | Change |\n");
            output.push_str("|------|------|----------|----------|---------|--------|\n");

            for reg in &report.regressions {
                output.push_str(&format!(
                    "| {} | {:?} | {:?} | {:.1} | {:.1} | {:+.1}% |\n",
                    reg.case_id,
                    reg.regression_type,
                    reg.severity,
                    reg.baseline_value,
                    reg.current_value,
                    reg.change_percent
                ));
            }

            output.push_str("\n### Details\n\n");
            for reg in &report.regressions {
                output.push_str(&format!("- **{}**: {}\n", reg.case_id, reg.description));
            }
        }

        output
    }

    // === JSON formatters ===

    fn summary_json(&self, summary: &BenchmarkSummary) -> String {
        serde_json::to_string_pretty(summary).unwrap_or_else(|_| "{}".to_string())
    }

    fn results_json(&self, results: &[BenchmarkResult]) -> String {
        serde_json::to_string_pretty(results).unwrap_or_else(|_| "[]".to_string())
    }

    fn regression_json(&self, report: &RegressionReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
    }

    // === CSV formatters ===

    fn summary_csv(&self, summary: &BenchmarkSummary) -> String {
        let mut output = String::new();
        output.push_str(
            "tier,total,passed,failed,pass_rate,avg_duration_ms,p50_duration_ms,p95_duration_ms\n",
        );

        for tier in &summary.tiers {
            output.push_str(&format!(
                "{},{},{},{},{:.2},{},{},{}\n",
                tier.tier.number(),
                tier.total,
                tier.passed,
                tier.failed,
                tier.pass_rate,
                tier.avg_duration_ms,
                tier.p50_duration_ms,
                tier.p95_duration_ms
            ));
        }

        output
    }

    fn results_csv(&self, results: &[BenchmarkResult]) -> String {
        let mut output = String::new();
        output.push_str("case_id,model,passed,duration_ms,tool_calls,error\n");

        for result in results {
            let error = result.error.as_deref().unwrap_or("");
            output.push_str(&format!(
                "{},{},{},{},{},\"{}\"\n",
                result.case_id,
                result.model,
                result.passed,
                result.duration_ms,
                result.tool_calls.len(),
                error.replace('"', "\"\"")
            ));
        }

        output
    }

    fn regression_csv(&self, report: &RegressionReport) -> String {
        let mut output = String::new();
        output.push_str("case_id,type,severity,baseline,current,change_percent\n");

        for reg in &report.regressions {
            output.push_str(&format!(
                "{},{:?},{:?},{:.2},{:.2},{:.2}\n",
                reg.case_id,
                reg.regression_type,
                reg.severity,
                reg.baseline_value,
                reg.current_value,
                reg.change_percent
            ));
        }

        output
    }

    // === Terminal formatters ===

    fn summary_terminal(&self, summary: &BenchmarkSummary) -> String {
        let mut output = String::new();

        let status = if summary.overall_pass_rate >= 90.0 {
            "✅"
        } else if summary.overall_pass_rate >= 70.0 {
            "⚠️"
        } else {
            "❌"
        };

        output.push_str(&format!(
            "\n{} Benchmark Summary: {}/{} passed ({:.1}%)\n",
            status, summary.total_passed, summary.total_cases, summary.overall_pass_rate
        ));
        output.push_str(&format!(
            "   Model: {} | Duration: {}ms\n\n",
            summary.model, summary.total_duration_ms
        ));

        for tier in &summary.tiers {
            let tier_status = if tier.pass_rate >= 90.0 {
                "✅"
            } else if tier.pass_rate >= 70.0 {
                "⚠️"
            } else {
                "❌"
            };
            output.push_str(&format!(
                "   {} {}: {}/{} ({:.1}%) | avg: {}ms\n",
                tier_status,
                tier.tier.name(),
                tier.passed,
                tier.total,
                tier.pass_rate,
                tier.avg_duration_ms
            ));
        }

        output.push('\n');
        output
    }

    fn results_terminal(&self, results: &[BenchmarkResult]) -> String {
        let mut output = String::new();

        for result in results {
            let status = if result.passed { "✅" } else { "❌" };
            output.push_str(&format!(
                "{} {} | {}ms | {} tools\n",
                status,
                result.case_id,
                result.duration_ms,
                result.tool_calls.len()
            ));

            if let Some(error) = &result.error {
                output.push_str(&format!("   Error: {}\n", error));
            }
        }

        output
    }

    fn regression_terminal(&self, report: &RegressionReport) -> String {
        let mut output = String::new();

        let status = if report.passed {
            "✅ PASSED"
        } else {
            "❌ FAILED"
        };
        output.push_str(&format!(
            "\n{} Regression Check: {} vs {}\n",
            status, report.model, report.baseline_model
        ));
        output.push_str(&format!(
            "   Compared: {} cases | Regressed: {}\n\n",
            report.cases_compared, report.cases_regressed
        ));

        if report.regressions.is_empty() {
            output.push_str("   No regressions detected.\n");
        } else {
            for reg in &report.regressions {
                let severity_icon = match reg.severity {
                    crate::baseline::RegressionSeverity::Minor => "⚠️",
                    crate::baseline::RegressionSeverity::Moderate => "🟠",
                    crate::baseline::RegressionSeverity::Severe => "🔴",
                };
                output.push_str(&format!("   {} {}\n", severity_icon, reg.description));
            }
        }

        output.push('\n');
        output
    }
}

impl Default for Reporter {
    fn default() -> Self {
        Self::new(OutputFormat::Terminal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BenchmarkSummary, Tier, TierSummary};
    use chrono::Utc;

    fn make_summary() -> BenchmarkSummary {
        BenchmarkSummary {
            model: "test-model".to_string(),
            timestamp: Utc::now(),
            tiers: vec![TierSummary {
                tier: Tier::Tier1,
                total: 10,
                passed: 9,
                failed: 1,
                pass_rate: 90.0,
                avg_duration_ms: 100,
                p50_duration_ms: 95,
                p95_duration_ms: 150,
            }],
            total_cases: 10,
            total_passed: 9,
            total_failed: 1,
            overall_pass_rate: 90.0,
            total_duration_ms: 1000,
        }
    }

    #[test]
    fn test_markdown_output() {
        let reporter = Reporter::new(OutputFormat::Markdown);
        let summary = make_summary();
        let output = reporter.summary(&summary);

        assert!(output.contains("# Benchmark Summary"));
        assert!(output.contains("test-model"));
        assert!(output.contains("90.0%"));
    }

    #[test]
    fn test_json_output() {
        let reporter = Reporter::new(OutputFormat::Json);
        let summary = make_summary();
        let output = reporter.summary(&summary);

        assert!(output.contains("\"model\":"));
        assert!(output.contains("\"total_cases\":"));
    }

    #[test]
    fn test_csv_output() {
        let reporter = Reporter::new(OutputFormat::Csv);
        let summary = make_summary();
        let output = reporter.summary(&summary);

        assert!(output.contains("tier,total,passed"));
        assert!(output.contains("1,10,9"));
    }

    #[test]
    fn test_format_parsing() {
        assert_eq!(
            "markdown".parse::<OutputFormat>().unwrap(),
            OutputFormat::Markdown
        );
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("csv".parse::<OutputFormat>().unwrap(), OutputFormat::Csv);
        assert_eq!(
            "terminal".parse::<OutputFormat>().unwrap(),
            OutputFormat::Terminal
        );
    }
}
