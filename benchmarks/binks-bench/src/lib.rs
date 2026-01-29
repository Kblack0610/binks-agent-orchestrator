//! Binks Benchmark Harness
//!
//! A comprehensive benchmarking system for validating Binks agent capabilities
//! across different tiers of complexity.
//!
//! ## Tiers
//!
//! - **Tier 1**: Simple single-tool tasks (e.g., read a file, list directory)
//! - **Tier 2**: Multi-step sequential tasks (e.g., search then read)
//! - **Tier 3**: Complex reasoning tasks (e.g., code analysis, multi-file investigation)
//! - **Platform**: Real-world platform repo tasks
//!
//! ## Usage
//!
//! ```rust,ignore
//! use binks_bench::{BenchmarkCase, BenchmarkRunner, Tier};
//!
//! let cases = binks_bench::cases::tier1::all_cases();
//! let runner = BenchmarkRunner::new("http://localhost:11434", "llama3.1:8b");
//! let results = runner.run_all(&cases).await?;
//! ```

pub mod baseline;
pub mod cases;
pub mod collector;
pub mod reporter;
pub mod runner;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Benchmark tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Simple single-tool tasks
    Tier1,
    /// Multi-step sequential tasks
    Tier2,
    /// Complex reasoning tasks
    Tier3,
    /// Platform-specific tasks
    Platform,
}

impl Tier {
    pub fn name(&self) -> &'static str {
        match self {
            Tier::Tier1 => "Tier 1 (Simple)",
            Tier::Tier2 => "Tier 2 (Multi-step)",
            Tier::Tier3 => "Tier 3 (Complex)",
            Tier::Platform => "Platform",
        }
    }

    pub fn number(&self) -> u8 {
        match self {
            Tier::Tier1 => 1,
            Tier::Tier2 => 2,
            Tier::Tier3 => 3,
            Tier::Platform => 4,
        }
    }
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Success criteria for a benchmark case
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SuccessCriteria {
    /// Output must contain the specified text
    ContainsText { text: String },
    /// Specific tools must have been called
    ToolsCalled { tools: Vec<String> },
    /// No errors occurred during execution
    NoErrors,
    /// All of the specified criteria must pass
    All { criteria: Vec<SuccessCriteria> },
    /// Any of the specified criteria must pass
    Any { criteria: Vec<SuccessCriteria> },
}

impl SuccessCriteria {
    /// Create a ContainsText criterion
    pub fn contains_text(text: impl Into<String>) -> Self {
        Self::ContainsText { text: text.into() }
    }

    /// Create a ToolsCalled criterion
    pub fn tools_called(tools: Vec<impl Into<String>>) -> Self {
        Self::ToolsCalled {
            tools: tools.into_iter().map(Into::into).collect(),
        }
    }

    /// Create a NoErrors criterion
    pub fn no_errors() -> Self {
        Self::NoErrors
    }

    /// Create an All criterion
    pub fn all(criteria: Vec<SuccessCriteria>) -> Self {
        Self::All { criteria }
    }

    /// Create an Any criterion
    pub fn any(criteria: Vec<SuccessCriteria>) -> Self {
        Self::Any { criteria }
    }

    /// Check if the criteria is satisfied by the result
    pub fn is_satisfied(&self, result: &BenchmarkResult) -> bool {
        match self {
            SuccessCriteria::ContainsText { text } => result.output.contains(text),
            SuccessCriteria::ToolsCalled { tools } => {
                let called: Vec<&str> = result.tool_calls.iter().map(|t| t.tool.as_str()).collect();
                tools.iter().all(|t| called.contains(&t.as_str()))
            }
            SuccessCriteria::NoErrors => result.error.is_none(),
            SuccessCriteria::All { criteria } => criteria.iter().all(|c| c.is_satisfied(result)),
            SuccessCriteria::Any { criteria } => criteria.iter().any(|c| c.is_satisfied(result)),
        }
    }
}

/// A benchmark test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkCase {
    /// Unique identifier for the case
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Tier classification
    pub tier: Tier,
    /// The prompt to send to the agent
    pub prompt: String,
    /// Tools that SHOULD be called (for validation)
    pub expected_tools: Vec<String>,
    /// Tools that MUST NOT be called (for validation)
    #[serde(default)]
    pub forbidden_tools: Vec<String>,
    /// Criteria for determining success
    pub success_criteria: SuccessCriteria,
    /// Maximum time allowed for execution
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    /// Optional MCP server filter
    #[serde(default)]
    pub servers: Option<Vec<String>>,
    /// Optional description of what the test validates
    #[serde(default)]
    pub description: Option<String>,
}

impl BenchmarkCase {
    /// Create a new benchmark case builder
    pub fn builder(id: impl Into<String>, prompt: impl Into<String>) -> BenchmarkCaseBuilder {
        BenchmarkCaseBuilder::new(id, prompt)
    }
}

/// Builder for creating benchmark cases
pub struct BenchmarkCaseBuilder {
    id: String,
    name: Option<String>,
    tier: Tier,
    prompt: String,
    expected_tools: Vec<String>,
    forbidden_tools: Vec<String>,
    success_criteria: SuccessCriteria,
    timeout: Duration,
    servers: Option<Vec<String>>,
    description: Option<String>,
}

impl BenchmarkCaseBuilder {
    pub fn new(id: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            tier: Tier::Tier1,
            prompt: prompt.into(),
            expected_tools: Vec::new(),
            forbidden_tools: Vec::new(),
            success_criteria: SuccessCriteria::NoErrors,
            timeout: Duration::from_secs(60),
            servers: None,
            description: None,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn tier(mut self, tier: Tier) -> Self {
        self.tier = tier;
        self
    }

    pub fn expected_tools(mut self, tools: Vec<impl Into<String>>) -> Self {
        self.expected_tools = tools.into_iter().map(Into::into).collect();
        self
    }

    pub fn forbidden_tools(mut self, tools: Vec<impl Into<String>>) -> Self {
        self.forbidden_tools = tools.into_iter().map(Into::into).collect();
        self
    }

    pub fn success_criteria(mut self, criteria: SuccessCriteria) -> Self {
        self.success_criteria = criteria;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn servers(mut self, servers: Vec<impl Into<String>>) -> Self {
        self.servers = Some(servers.into_iter().map(Into::into).collect());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn build(self) -> BenchmarkCase {
        BenchmarkCase {
            id: self.id.clone(),
            name: self.name.unwrap_or_else(|| self.id.clone()),
            tier: self.tier,
            prompt: self.prompt,
            expected_tools: self.expected_tools,
            forbidden_tools: self.forbidden_tools,
            success_criteria: self.success_criteria,
            timeout: self.timeout,
            servers: self.servers,
            description: self.description,
        }
    }
}

/// Metrics for a single tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallMetric {
    /// Tool name (e.g., "mcp__filesystem__read_file")
    pub tool: String,
    /// Server name (e.g., "filesystem")
    pub server: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Whether the call succeeded
    pub success: bool,
    /// Error type if failed
    #[serde(default)]
    pub error_type: Option<String>,
    /// Timestamp of the call
    pub timestamp: DateTime<Utc>,
}

/// Result of running a benchmark case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Case ID
    pub case_id: String,
    /// Model used
    pub model: String,
    /// Whether the benchmark passed
    pub passed: bool,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Tool calls made during execution
    pub tool_calls: Vec<ToolCallMetric>,
    /// Number of iterations (for averaged results)
    pub iterations: usize,
    /// Error message if failed
    #[serde(default)]
    pub error: Option<String>,
    /// Agent output
    pub output: String,
    /// Timestamp of the run
    pub timestamp: DateTime<Utc>,
    /// Expected tools that were NOT called
    #[serde(default)]
    pub missing_tools: Vec<String>,
    /// Forbidden tools that WERE called
    #[serde(default)]
    pub forbidden_tools_called: Vec<String>,
}

impl BenchmarkResult {
    /// Check if any expected tools were missing
    pub fn has_missing_tools(&self) -> bool {
        !self.missing_tools.is_empty()
    }

    /// Check if any forbidden tools were called
    pub fn has_forbidden_tools(&self) -> bool {
        !self.forbidden_tools_called.is_empty()
    }
}

/// Summary of benchmark results for a tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierSummary {
    pub tier: Tier,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate: f64,
    pub avg_duration_ms: u64,
    pub p50_duration_ms: u64,
    pub p95_duration_ms: u64,
}

/// Complete benchmark run summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub model: String,
    pub timestamp: DateTime<Utc>,
    pub tiers: Vec<TierSummary>,
    pub total_cases: usize,
    pub total_passed: usize,
    pub total_failed: usize,
    pub overall_pass_rate: f64,
    pub total_duration_ms: u64,
}

// Re-export important items
pub use baseline::{Baseline, RegressionReport};
pub use collector::BenchmarkCollector;
pub use reporter::{OutputFormat, Reporter};
pub use runner::{BenchmarkRunner, RunnerConfig};

/// Humantime serde support for Duration
mod humantime_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
