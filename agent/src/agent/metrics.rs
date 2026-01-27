//! MCP tool call metrics and observability
//!
//! Tracks per-server metrics for tool calls including success/failure counts,
//! durations, and error classification. Metrics are in-memory only and reset
//! on agent restart.

use std::collections::HashMap;
use std::time::Instant;

use serde::Serialize;

// ============================================================================
// Error Classification
// ============================================================================

/// Classification of tool call errors for observability
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallError {
    /// Agent-level timeout fired (tokio::time::timeout)
    Timeout,
    /// Daemon socket not available
    ConnectionRefused,
    /// MCP process died mid-call
    ServerCrashed,
    /// MCP server returned an error response
    ToolError(String),
    /// Serialization/protocol/transport issues
    TransportError(String),
}

impl ToolCallError {
    /// Classify an error string into a ToolCallError variant
    pub fn classify(error: &str) -> Self {
        let lower = error.to_lowercase();
        if lower.contains("timed out") || lower.contains("deadline has elapsed") {
            Self::Timeout
        } else if lower.contains("connection refused")
            || lower.contains("no such file or directory")
            || lower.contains("failed to connect")
        {
            Self::ConnectionRefused
        } else if lower.contains("broken pipe")
            || lower.contains("connection reset")
            || lower.contains("aborted")
            || lower.contains("process exited")
        {
            Self::ServerCrashed
        } else if lower.contains("failed to parse")
            || lower.contains("serialization")
            || lower.contains("protocol")
        {
            Self::TransportError(error.to_string())
        } else {
            Self::ToolError(error.to_string())
        }
    }

    /// Get a short label for the error type
    pub fn label(&self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::ConnectionRefused => "connection_refused",
            Self::ServerCrashed => "server_crashed",
            Self::ToolError(_) => "tool_error",
            Self::TransportError(_) => "transport_error",
        }
    }
}

impl std::fmt::Display for ToolCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "timeout"),
            Self::ConnectionRefused => write!(f, "connection refused"),
            Self::ServerCrashed => write!(f, "server crashed"),
            Self::ToolError(msg) => write!(f, "tool error: {}", msg),
            Self::TransportError(msg) => write!(f, "transport error: {}", msg),
        }
    }
}

// ============================================================================
// Per-Server Metrics
// ============================================================================

/// Metrics for a single MCP server
#[derive(Debug, Clone, Serialize)]
pub struct ServerMetrics {
    pub server_name: String,
    pub total_calls: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub timeout_count: u64,
    pub total_duration_ms: u64,
    pub last_error: Option<String>,
    #[serde(skip)]
    pub last_call_at: Option<Instant>,
    /// Average duration in milliseconds (computed)
    pub avg_duration_ms: u64,
}

impl ServerMetrics {
    fn new(name: &str) -> Self {
        Self {
            server_name: name.to_string(),
            total_calls: 0,
            success_count: 0,
            error_count: 0,
            timeout_count: 0,
            total_duration_ms: 0,
            last_error: None,
            last_call_at: None,
            avg_duration_ms: 0,
        }
    }

    fn record_success(&mut self, duration_ms: u64) {
        self.total_calls += 1;
        self.success_count += 1;
        self.total_duration_ms += duration_ms;
        self.last_call_at = Some(Instant::now());
        self.update_avg();
    }

    fn record_error(&mut self, duration_ms: u64, error: &ToolCallError) {
        self.total_calls += 1;
        self.error_count += 1;
        self.total_duration_ms += duration_ms;
        self.last_error = Some(error.to_string());
        self.last_call_at = Some(Instant::now());
        if matches!(error, ToolCallError::Timeout) {
            self.timeout_count += 1;
        }
        self.update_avg();
    }

    fn update_avg(&mut self) {
        if self.total_calls > 0 {
            self.avg_duration_ms = self.total_duration_ms / self.total_calls;
        }
    }

    /// Success rate as a percentage (0.0 - 100.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 100.0;
        }
        (self.success_count as f64 / self.total_calls as f64) * 100.0
    }
}

// ============================================================================
// Metrics Tracker
// ============================================================================

/// Tracks metrics across all MCP servers
#[derive(Debug, Clone, Default)]
pub struct McpMetrics {
    servers: HashMap<String, ServerMetrics>,
}

impl McpMetrics {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    /// Record a successful tool call
    pub fn record_success(&mut self, server: &str, tool: &str, duration_ms: u64) {
        tracing::debug!(
            mcp.server = server,
            mcp.tool = tool,
            mcp.duration_ms = duration_ms,
            "Tool call succeeded"
        );
        self.servers
            .entry(server.to_string())
            .or_insert_with(|| ServerMetrics::new(server))
            .record_success(duration_ms);
    }

    /// Record a failed tool call
    pub fn record_error(
        &mut self,
        server: &str,
        tool: &str,
        duration_ms: u64,
        error: &ToolCallError,
    ) {
        tracing::warn!(
            mcp.server = server,
            mcp.tool = tool,
            mcp.duration_ms = duration_ms,
            mcp.error_type = error.label(),
            "Tool call failed: {}",
            error
        );
        self.servers
            .entry(server.to_string())
            .or_insert_with(|| ServerMetrics::new(server))
            .record_error(duration_ms, error);
    }

    /// Record a timeout (convenience wrapper)
    pub fn record_timeout(&mut self, server: &str, tool: &str, duration_ms: u64) {
        self.record_error(server, tool, duration_ms, &ToolCallError::Timeout);
    }

    /// Get metrics for a specific server
    pub fn get_server(&self, server: &str) -> Option<&ServerMetrics> {
        self.servers.get(server)
    }

    /// Get metrics for all servers
    pub fn all_servers(&self) -> Vec<&ServerMetrics> {
        self.servers.values().collect()
    }

    /// Get a snapshot of all metrics (cloned, for serialization)
    pub fn snapshot(&self) -> Vec<ServerMetrics> {
        self.servers.values().cloned().collect()
    }

    /// Get total calls across all servers
    pub fn total_calls(&self) -> u64 {
        self.servers.values().map(|s| s.total_calls).sum()
    }

    /// Get total errors across all servers
    pub fn total_errors(&self) -> u64 {
        self.servers.values().map(|s| s.error_count).sum()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        assert!(matches!(
            ToolCallError::classify("Tool test timed out after 60s"),
            ToolCallError::Timeout
        ));
        assert!(matches!(
            ToolCallError::classify("Failed to connect to daemon: connection refused"),
            ToolCallError::ConnectionRefused
        ));
        assert!(matches!(
            ToolCallError::classify("The operation was aborted"),
            ToolCallError::ServerCrashed
        ));
        assert!(matches!(
            ToolCallError::classify("Failed to parse daemon response"),
            ToolCallError::TransportError(_)
        ));
        assert!(matches!(
            ToolCallError::classify("Some random error"),
            ToolCallError::ToolError(_)
        ));
    }

    #[test]
    fn test_metrics_tracking() {
        let mut metrics = McpMetrics::new();

        metrics.record_success("server1", "tool_a", 100);
        metrics.record_success("server1", "tool_b", 200);
        metrics.record_error(
            "server1",
            "tool_a",
            50,
            &ToolCallError::ToolError("oops".into()),
        );
        metrics.record_timeout("server2", "tool_c", 60000);

        let s1 = metrics.get_server("server1").unwrap();
        assert_eq!(s1.total_calls, 3);
        assert_eq!(s1.success_count, 2);
        assert_eq!(s1.error_count, 1);
        assert_eq!(s1.timeout_count, 0);
        assert_eq!(s1.total_duration_ms, 350);
        assert!(s1.last_error.is_some());

        let s2 = metrics.get_server("server2").unwrap();
        assert_eq!(s2.total_calls, 1);
        assert_eq!(s2.error_count, 1);
        assert_eq!(s2.timeout_count, 1);

        assert_eq!(metrics.total_calls(), 4);
        assert_eq!(metrics.total_errors(), 2);
    }

    #[test]
    fn test_success_rate() {
        let mut metrics = McpMetrics::new();
        metrics.record_success("s1", "t1", 100);
        metrics.record_success("s1", "t1", 100);
        metrics.record_error("s1", "t1", 50, &ToolCallError::Timeout);

        let s1 = metrics.get_server("s1").unwrap();
        let rate = s1.success_rate();
        assert!((rate - 66.66).abs() < 1.0); // ~66.67%
    }

    #[test]
    fn test_metrics_serialization() {
        let mut metrics = McpMetrics::new();
        metrics.record_success("test_server", "test_tool", 150);

        let snapshot = metrics.snapshot();
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("test_server"));
        assert!(json.contains("\"total_calls\":1"));
    }
}
