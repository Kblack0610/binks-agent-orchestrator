//! Agent events for real-time visibility
//!
//! This module defines events emitted by the agent during execution.
//! These events can be consumed by CLI output, WebSocket handlers,
//! or any other subscriber.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// ============================================================================
// Agent Events
// ============================================================================

/// Events emitted by the agent during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Agent is starting to process a message
    ProcessingStart {
        /// The user's input message
        message: String,
    },

    /// Tool execution is starting
    ToolStart {
        /// Tool name
        name: String,
        /// Tool arguments
        arguments: serde_json::Value,
    },

    /// Tool execution completed
    ToolComplete {
        /// Tool name
        name: String,
        /// Tool result (may be truncated for display)
        result: String,
        /// Execution duration
        #[serde(with = "duration_millis")]
        duration: Duration,
        /// Whether the tool call failed
        is_error: bool,
    },

    /// Streaming token received
    Token {
        /// The token content
        content: String,
    },

    /// Thinking/reasoning content (for models that support it)
    Thinking {
        /// The thinking content
        content: String,
    },

    /// Agent iteration (for visibility into the tool-calling loop)
    Iteration {
        /// Current iteration number
        number: usize,
        /// Number of tool calls in this iteration
        tool_calls: usize,
    },

    /// Final response ready
    ResponseComplete {
        /// The final response content
        content: String,
        /// Total number of iterations
        iterations: usize,
        /// Total duration
        #[serde(with = "duration_millis")]
        total_duration: Duration,
    },

    /// An error occurred
    Error {
        /// Error message
        message: String,
    },
}

/// Serialize Duration as milliseconds
mod duration_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

// ============================================================================
// Event Channel
// ============================================================================

/// Sender for agent events
pub type EventSender = mpsc::UnboundedSender<AgentEvent>;

/// Receiver for agent events
pub type EventReceiver = mpsc::UnboundedReceiver<AgentEvent>;

/// Create a new event channel
pub fn event_channel() -> (EventSender, EventReceiver) {
    mpsc::unbounded_channel()
}

// ============================================================================
// Event Sender Helper
// ============================================================================

/// Helper struct for sending events with a consistent API
#[derive(Clone)]
pub struct AgentEventSender {
    sender: Option<EventSender>,
}

impl Default for AgentEventSender {
    fn default() -> Self {
        Self::none()
    }
}

impl AgentEventSender {
    /// Create with an actual sender
    pub fn new(sender: EventSender) -> Self {
        Self {
            sender: Some(sender),
        }
    }

    /// Create a no-op sender (events are discarded)
    pub fn none() -> Self {
        Self { sender: None }
    }

    /// Check if events will be sent
    pub fn is_active(&self) -> bool {
        self.sender.is_some()
    }

    /// Send an event (silently fails if no sender or receiver dropped)
    pub fn send(&self, event: AgentEvent) {
        if let Some(ref sender) = self.sender {
            // Ignore send errors - receiver may have dropped
            let _ = sender.send(event);
        }
    }

    /// Send processing start event
    pub fn processing_start(&self, message: &str) {
        self.send(AgentEvent::ProcessingStart {
            message: message.to_string(),
        });
    }

    /// Send tool start event
    pub fn tool_start(&self, name: &str, arguments: &serde_json::Value) {
        self.send(AgentEvent::ToolStart {
            name: name.to_string(),
            arguments: arguments.clone(),
        });
    }

    /// Send tool complete event
    pub fn tool_complete(&self, name: &str, result: &str, duration: Duration, is_error: bool) {
        self.send(AgentEvent::ToolComplete {
            name: name.to_string(),
            result: result.to_string(),
            duration,
            is_error,
        });
    }

    /// Send token event
    pub fn token(&self, content: &str) {
        self.send(AgentEvent::Token {
            content: content.to_string(),
        });
    }

    /// Send thinking event
    pub fn thinking(&self, content: &str) {
        self.send(AgentEvent::Thinking {
            content: content.to_string(),
        });
    }

    /// Send iteration event
    pub fn iteration(&self, number: usize, tool_calls: usize) {
        self.send(AgentEvent::Iteration { number, tool_calls });
    }

    /// Send response complete event
    pub fn response_complete(&self, content: &str, iterations: usize, total_duration: Duration) {
        self.send(AgentEvent::ResponseComplete {
            content: content.to_string(),
            iterations,
            total_duration,
        });
    }

    /// Send error event
    pub fn error(&self, message: &str) {
        self.send(AgentEvent::Error {
            message: message.to_string(),
        });
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_channel() {
        let (tx, mut rx) = event_channel();

        tx.send(AgentEvent::ToolStart {
            name: "test_tool".to_string(),
            arguments: serde_json::json!({"key": "value"}),
        })
        .unwrap();

        let event = rx.recv().await.unwrap();
        match event {
            AgentEvent::ToolStart { name, .. } => assert_eq!(name, "test_tool"),
            _ => panic!("Expected ToolStart event"),
        }
    }

    #[test]
    fn test_event_sender_helper() {
        let (tx, _rx) = event_channel();
        let sender = AgentEventSender::new(tx);

        assert!(sender.is_active());

        // These should not panic
        sender.tool_start("tool", &serde_json::json!({}));
        sender.tool_complete("tool", "result", Duration::from_millis(100), false);
        sender.token("hello");
        sender.thinking("hmm");
        sender.iteration(1, 2);
        sender.error("oops");
    }

    #[test]
    fn test_noop_sender() {
        let sender = AgentEventSender::none();
        assert!(!sender.is_active());

        // These should not panic even without a receiver
        sender.tool_start("tool", &serde_json::json!({}));
        sender.error("oops");
    }

    #[test]
    fn test_event_serialization() {
        let event = AgentEvent::ToolComplete {
            name: "test".to_string(),
            result: "ok".to_string(),
            duration: Duration::from_millis(123),
            is_error: false,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"tool_complete\""));
        assert!(json.contains("\"duration\":123"));

        // Deserialize back
        let parsed: AgentEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentEvent::ToolComplete { duration, .. } => {
                assert_eq!(duration.as_millis(), 123);
            }
            _ => panic!("Expected ToolComplete"),
        }
    }
}
