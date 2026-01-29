//! Benchmark metrics collector
//!
//! Consumes AgentEvent stream and collects metrics for benchmark analysis.

use crate::ToolCallMetric;
use agent::agent::AgentEvent;
use chrono::Utc;
use std::time::Instant;
use tokio::sync::mpsc;

/// Collected metrics from a benchmark run
#[derive(Debug, Clone)]
pub struct CollectedMetrics {
    /// Tool calls recorded during execution
    pub tool_calls: Vec<ToolCallMetric>,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Final output text
    pub output: String,
    /// Error if any
    pub error: Option<String>,
    /// Number of iterations
    pub iterations: usize,
}

/// Collector that consumes AgentEvent and builds metrics
pub struct BenchmarkCollector {
    tool_calls: Vec<ToolCallMetric>,
    start_time: Instant,
    output: String,
    error: Option<String>,
    iterations: usize,
}

impl BenchmarkCollector {
    /// Create a new collector
    pub fn new() -> Self {
        Self {
            tool_calls: Vec::new(),
            start_time: Instant::now(),
            output: String::new(),
            error: None,
            iterations: 0,
        }
    }

    /// Extract server name from tool name (e.g., "mcp__filesystem__read_file" -> "filesystem")
    fn extract_server(tool_name: &str) -> String {
        // Tool names are in format: mcp__{server}__{tool}
        if tool_name.starts_with("mcp__") {
            if let Some(rest) = tool_name.strip_prefix("mcp__") {
                if let Some(idx) = rest.find("__") {
                    return rest[..idx].to_string();
                }
            }
        }
        "unknown".to_string()
    }

    /// Collect metrics from an event receiver
    ///
    /// Consumes events until the channel closes or a terminal event is received.
    pub async fn collect(mut rx: mpsc::UnboundedReceiver<AgentEvent>) -> CollectedMetrics {
        let mut collector = Self::new();

        while let Some(event) = rx.recv().await {
            let is_complete = collector.process_event(event);
            if is_complete {
                break;
            }
        }

        collector.finalize()
    }

    /// Process a single event, returns true if this is a terminal event
    fn process_event(&mut self, event: AgentEvent) -> bool {
        match event {
            AgentEvent::ToolStart { name, .. } => {
                tracing::debug!(tool = %name, "Tool started");
                false
            }
            AgentEvent::ToolComplete {
                name,
                duration,
                is_error,
                error_type,
                ..
            } => {
                let server = Self::extract_server(&name);
                let metric = ToolCallMetric {
                    tool: name.clone(),
                    server,
                    duration_ms: duration.as_millis() as u64,
                    success: !is_error,
                    error_type,
                    timestamp: Utc::now(),
                };
                self.tool_calls.push(metric);
                tracing::debug!(
                    tool = %name,
                    duration_ms = duration.as_millis(),
                    success = !is_error,
                    "Tool completed"
                );
                false
            }
            AgentEvent::Token { content } => {
                self.output.push_str(&content);
                false
            }
            AgentEvent::Iteration {
                number,
                tool_calls: _,
            } => {
                self.iterations = number;
                false
            }
            AgentEvent::ResponseComplete {
                content,
                iterations,
                ..
            } => {
                // Use the final content as output if we didn't get tokens
                if self.output.is_empty() {
                    self.output = content;
                }
                self.iterations = iterations;
                tracing::debug!(iterations = iterations, "Response complete");
                true // Terminal event
            }
            AgentEvent::Error { message } => {
                self.error = Some(message.clone());
                tracing::warn!(error = %message, "Error during benchmark");
                true // Terminal event
            }
            _ => false,
        }
    }

    /// Finalize collection and return metrics
    fn finalize(self) -> CollectedMetrics {
        CollectedMetrics {
            tool_calls: self.tool_calls,
            duration_ms: self.start_time.elapsed().as_millis() as u64,
            output: self.output,
            error: self.error,
            iterations: self.iterations,
        }
    }
}

impl Default for BenchmarkCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_collector_basic() {
        let (tx, rx) = mpsc::unbounded_channel();

        // Send some events
        tx.send(AgentEvent::ToolStart {
            name: "mcp__filesystem__read_file".to_string(),
            arguments: serde_json::json!({}),
        })
        .unwrap();

        tx.send(AgentEvent::ToolComplete {
            name: "mcp__filesystem__read_file".to_string(),
            result: "file contents".to_string(),
            duration: Duration::from_millis(100),
            is_error: false,
            error_type: None,
        })
        .unwrap();

        tx.send(AgentEvent::Token {
            content: "Hello, world!".to_string(),
        })
        .unwrap();

        tx.send(AgentEvent::ResponseComplete {
            content: "Hello, world!".to_string(),
            iterations: 1,
            total_duration: Duration::from_millis(200),
        })
        .unwrap();

        let metrics = BenchmarkCollector::collect(rx).await;

        assert_eq!(metrics.tool_calls.len(), 1);
        assert_eq!(metrics.tool_calls[0].tool, "mcp__filesystem__read_file");
        assert_eq!(metrics.tool_calls[0].server, "filesystem");
        assert!(metrics.tool_calls[0].success);
        assert_eq!(metrics.output, "Hello, world!");
        assert!(metrics.error.is_none());
        assert_eq!(metrics.iterations, 1);
    }

    #[test]
    fn test_extract_server() {
        assert_eq!(
            BenchmarkCollector::extract_server("mcp__filesystem__read_file"),
            "filesystem"
        );
        assert_eq!(
            BenchmarkCollector::extract_server("mcp__sysinfo__get_os_info"),
            "sysinfo"
        );
        assert_eq!(
            BenchmarkCollector::extract_server("mcp__github-gh__gh_pr_list"),
            "github-gh"
        );
        assert_eq!(
            BenchmarkCollector::extract_server("unknown_tool"),
            "unknown"
        );
    }
}
