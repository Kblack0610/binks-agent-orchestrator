//! Output abstraction for CLI and other consumers
//!
//! This module provides a trait-based output system that decouples event emission
//! from display logic. Different implementations can handle terminal output,
//! plain text (for pipes/CI), or other formats.

use std::time::Duration;

mod terminal;
mod plain;

pub use terminal::TerminalOutput;
pub use plain::PlainOutput;

// ============================================================================
// Output Events
// ============================================================================

/// Events that can be displayed to the user
#[derive(Debug, Clone)]
pub enum OutputEvent {
    /// Plain text message
    Text(String),

    /// Tool execution started
    ToolStart {
        name: String,
        arguments: serde_json::Value,
    },

    /// Tool execution completed
    ToolComplete {
        name: String,
        result: String,
        duration: Duration,
        is_error: bool,
    },

    /// Thinking/reasoning content (for models that support it)
    Thinking(String),

    /// Streaming token
    Token(String),

    /// Progress indicator
    Progress {
        message: String,
        done: bool,
    },

    /// Status message (informational)
    Status(String),

    /// Error message
    Error(String),

    /// Warning message
    Warning(String),

    /// System message (dimmed, for internal info)
    System(String),

    /// New line / separator
    NewLine,
}

// ============================================================================
// Output Writer Trait
// ============================================================================

/// Trait for writing output events
///
/// Implementations handle how events are displayed. This decouples
/// the agent's event emission from the display logic.
pub trait OutputWriter: Send + Sync {
    /// Write an output event
    fn write(&self, event: OutputEvent);

    /// Flush any buffered output
    fn flush(&self);

    /// Whether this writer supports streaming tokens
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Whether this writer supports colors/formatting
    fn supports_colors(&self) -> bool {
        false
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a default output writer based on environment
pub fn default_output() -> Box<dyn OutputWriter> {
    // Check if stdout is a terminal
    if atty::is(atty::Stream::Stdout) {
        Box::new(TerminalOutput::new())
    } else {
        Box::new(PlainOutput::new())
    }
}

/// Create a terminal output writer
pub fn terminal_output() -> TerminalOutput {
    TerminalOutput::new()
}

/// Create a plain output writer
pub fn plain_output() -> PlainOutput {
    PlainOutput::new()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Mock output writer for testing
    struct MockOutput {
        events: Arc<Mutex<Vec<OutputEvent>>>,
    }

    impl MockOutput {
        fn new() -> (Self, Arc<Mutex<Vec<OutputEvent>>>) {
            let events = Arc::new(Mutex::new(Vec::new()));
            (Self { events: events.clone() }, events)
        }
    }

    impl OutputWriter for MockOutput {
        fn write(&self, event: OutputEvent) {
            self.events.lock().unwrap().push(event);
        }

        fn flush(&self) {}
    }

    #[test]
    fn test_mock_output() {
        let (mock, events) = MockOutput::new();

        mock.write(OutputEvent::Text("Hello".to_string()));
        mock.write(OutputEvent::Status("Working...".to_string()));

        let captured = events.lock().unwrap();
        assert_eq!(captured.len(), 2);

        match &captured[0] {
            OutputEvent::Text(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected Text event"),
        }
    }

    #[test]
    fn test_output_event_variants() {
        // Ensure all variants can be created
        let events = vec![
            OutputEvent::Text("test".into()),
            OutputEvent::ToolStart {
                name: "tool".into(),
                arguments: serde_json::json!({}),
            },
            OutputEvent::ToolComplete {
                name: "tool".into(),
                result: "ok".into(),
                duration: Duration::from_millis(100),
                is_error: false,
            },
            OutputEvent::Thinking("hmm".into()),
            OutputEvent::Token("tok".into()),
            OutputEvent::Progress {
                message: "loading".into(),
                done: false,
            },
            OutputEvent::Status("status".into()),
            OutputEvent::Error("err".into()),
            OutputEvent::Warning("warn".into()),
            OutputEvent::System("sys".into()),
            OutputEvent::NewLine,
        ];

        assert_eq!(events.len(), 11);
    }
}
