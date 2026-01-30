//! Response types for self-healing MCP tools

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Health report for a single run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunHealthReport {
    pub run_id: String,
    pub health_score: f64, // 0.0 to 100.0
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub tool_reliability: f64,
    pub resource_efficiency: f64,
    pub issues: Vec<String>,
    pub trend: Trend,
}

/// Detected error pattern across multiple runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub id: String,
    pub error_type: String,
    pub tool_name: Option<String>,
    pub occurrences: usize,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub affected_runs: Vec<String>,
    pub correlation_score: f64, // 0.0 to 1.0
    pub suggested_fix: Option<String>,
}

/// Success metrics for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetric {
    pub agent_name: String,
    pub total_runs: usize,
    pub successful_runs: usize,
    pub failed_runs: usize,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub trend: Trend,
}

/// Tool reliability metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolReliability {
    pub tool_name: String,
    pub total_calls: usize,
    pub successful_calls: usize,
    pub failed_calls: usize,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub common_errors: Vec<String>,
}

/// Improvement proposal based on detected patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementProposal {
    pub id: String,
    pub pattern_id: String,
    pub category: String, // prompt, workflow, agent, tool, other
    pub priority: Priority,
    pub description: String,
    pub suggested_changes: String,
    pub expected_impact: String,
    pub status: ImprovementStatus,
    pub created_at: DateTime<Utc>,
}

/// Verification result after applying an improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub improvement_id: String,
    pub expected_impact: String,
    pub actual_impact: String,
    pub success_rate_before: f64,
    pub success_rate_after: f64,
    pub runs_analyzed: usize,
    pub recommendation: String, // Keep, Rollback, Adjust
    pub verified_at: DateTime<Utc>,
}

/// Overall system health dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDashboard {
    pub overall_health_score: f64,
    pub total_runs: usize,
    pub success_rate: f64,
    pub active_patterns: usize,
    pub pending_improvements: usize,
    pub applied_improvements: usize,
    pub top_agents: Vec<AgentMetric>,
    pub top_failing_tools: Vec<ToolReliability>,
    pub recent_trends: Vec<TrendSummary>,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Trend {
    Improving,
    Degrading,
    Stable,
}

/// Priority level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
    Urgent,
}

/// Improvement status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImprovementStatus {
    Proposed,
    Applied,
    Verified,
    Rejected,
}

/// Trend summary for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendSummary {
    pub metric: String,
    pub trend: Trend,
    pub change_percent: f64,
}
