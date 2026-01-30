//! Parameter types for self-healing MCP tools

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for analyze_run_health
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeRunHealthParams {
    /// Run ID or prefix (minimum 8 characters)
    pub run_id: String,

    /// Include detailed event analysis
    #[serde(default)]
    pub include_events: bool,
}

/// Parameters for detect_failure_patterns
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetectPatternsParams {
    /// Look back period (e.g., "-7d", "-30d")
    #[serde(default = "default_since")]
    pub since: String,

    /// Minimum occurrences to be considered a pattern
    #[serde(default = "default_min_occurrences")]
    pub min_occurrences: usize,

    /// Optional filter by error type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,

    /// Optional filter by tool name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

fn default_since() -> String {
    "-7d".to_string()
}

fn default_min_occurrences() -> usize {
    3
}

/// Parameters for compute_agent_metrics
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComputeAgentMetricsParams {
    /// Look back period (e.g., "-7d", "-30d")
    #[serde(default = "default_since")]
    pub since: String,

    /// Optional filter by specific agent name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
}

/// Parameters for compute_tool_reliability
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComputeToolReliabilityParams {
    /// Look back period (e.g., "-7d", "-30d")
    #[serde(default = "default_since")]
    pub since: String,

    /// Optional filter by tool name pattern
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_pattern: Option<String>,
}

/// Parameters for propose_improvement
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProposeImprovementParams {
    /// Pattern ID to generate improvement for
    pub pattern_id: String,

    /// Category: prompt, workflow, agent, tool, other
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Priority: low, medium, high, urgent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}

/// Parameters for test_improvement
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestImprovementParams {
    /// Improvement ID to test
    pub improvement_id: String,

    /// Test mode: simulation, canary, sandbox
    #[serde(default = "default_test_mode")]
    pub mode: String,
}

fn default_test_mode() -> String {
    "simulation".to_string()
}

/// Parameters for apply_improvement
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyImprovementParams {
    /// Improvement ID to apply
    pub improvement_id: String,

    /// Description of changes made
    pub changes_made: String,

    /// Optional commit hash if code was changed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_hash: Option<String>,
}

/// Parameters for verify_improvement
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VerifyImprovementParams {
    /// Improvement ID to verify
    pub improvement_id: String,

    /// Measurement window in days
    #[serde(default = "default_measurement_window")]
    pub measurement_window_days: usize,
}

fn default_measurement_window() -> usize {
    7
}

/// Parameters for get_health_dashboard
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetHealthDashboardParams {
    /// Look back period (e.g., "-7d", "-30d")
    #[serde(default = "default_since")]
    pub since: String,

    /// Number of top agents to include
    #[serde(default = "default_top_n")]
    pub top_agents: usize,

    /// Number of top failing tools to include
    #[serde(default = "default_top_n")]
    pub top_tools: usize,
}

fn default_top_n() -> usize {
    5
}
