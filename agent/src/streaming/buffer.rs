//! Stream buffer for accumulating streamed responses
//!
//! Handles buffering of streamed text and tool call detection.

use super::{StreamToolCall, StreamUsage, StreamingResult};

// ============================================================================
// Stream Chunk (parsed)
// ============================================================================

/// A parsed chunk from the stream
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Text content
    Text(String),
    /// Tool call detected
    ToolCall(StreamToolCall),
    /// Stream complete
    Done,
}

// ============================================================================
// Stream Buffer
// ============================================================================

/// Buffer for accumulating streamed responses
///
/// This handles:
/// - Accumulating text tokens
/// - Detecting and capturing tool calls
/// - Tracking completion status
#[derive(Debug, Default)]
pub struct StreamBuffer {
    /// Accumulated text content
    content: String,
    /// Detected tool calls
    tool_calls: Vec<StreamToolCall>,
    /// Whether stream is complete
    done: bool,
    /// Usage statistics
    usage: Option<StreamUsage>,
}

impl StreamBuffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self::default()
    }

    /// Push text content to the buffer
    pub fn push_text(&mut self, text: &str) {
        self.content.push_str(text);
    }

    /// Get the accumulated content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set tool calls
    pub fn set_tool_calls(&mut self, calls: Vec<StreamToolCall>) {
        self.tool_calls = calls;
    }

    /// Get tool calls
    pub fn tool_calls(&self) -> &[StreamToolCall] {
        &self.tool_calls
    }

    /// Mark stream as done
    pub fn set_done(&mut self, done: bool) {
        self.done = done;
    }

    /// Check if stream is done
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Set usage statistics
    pub fn set_usage(&mut self, prompt_tokens: u32, completion_tokens: u32) {
        self.usage = Some(StreamUsage {
            prompt_tokens,
            completion_tokens,
        });
    }

    /// Get usage statistics
    pub fn usage(&self) -> Option<&StreamUsage> {
        self.usage.as_ref()
    }

    /// Check if the buffer contains tool calls
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    /// Convert buffer into final result
    pub fn into_result(self) -> StreamingResult {
        StreamingResult {
            content: self.content,
            tool_calls: self.tool_calls,
            done: self.done,
            usage: self.usage,
        }
    }

    /// Reset the buffer for a new stream
    pub fn reset(&mut self) {
        self.content.clear();
        self.tool_calls.clear();
        self.done = false;
        self.usage = None;
    }
}

// ============================================================================
// JSON Tool Call Detection (fallback for models that embed JSON in content)
// ============================================================================

/// Attempt to detect tool calls embedded in JSON content
///
/// Some models (especially smaller ones) don't use proper tool call format
/// and instead emit JSON in the content. This tries to detect that pattern.
#[allow(dead_code)]
pub fn detect_embedded_tool_calls(content: &str) -> Option<Vec<StreamToolCall>> {
    let content = content.trim();

    // Look for JSON-like content that might be tool calls
    // Common patterns:
    // 1. {"name": "...", "arguments": {...}}
    // 2. [{"name": "...", "arguments": {...}}]
    // 3. <tool_call>{"name": "..."}</tool_call>

    // Try to find JSON object boundaries
    if let Some(start) = content.find('{') {
        // Find matching closing brace
        let mut depth = 0;
        let mut end = None;

        for (i, c) in content[start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(end) = end {
            let json_str = &content[start..end];

            // Try to parse as a tool call
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                // Check if it looks like a tool call
                if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
                    let arguments = value
                        .get("arguments")
                        .cloned()
                        .or_else(|| value.get("parameters").cloned())
                        .unwrap_or(serde_json::json!({}));

                    return Some(vec![StreamToolCall {
                        function: super::StreamFunction {
                            name: name.to_string(),
                            arguments,
                        },
                    }]);
                }
            }
        }
    }

    None
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_basic() {
        let mut buffer = StreamBuffer::new();
        assert!(buffer.content().is_empty());
        assert!(!buffer.is_done());

        buffer.push_text("Hello ");
        buffer.push_text("World");
        assert_eq!(buffer.content(), "Hello World");

        buffer.set_done(true);
        assert!(buffer.is_done());
    }

    #[test]
    fn test_buffer_tool_calls() {
        let mut buffer = StreamBuffer::new();
        assert!(!buffer.has_tool_calls());

        buffer.set_tool_calls(vec![StreamToolCall {
            function: super::super::StreamFunction {
                name: "test".to_string(),
                arguments: serde_json::json!({"arg": 1}),
            },
        }]);

        assert!(buffer.has_tool_calls());
        assert_eq!(buffer.tool_calls().len(), 1);
    }

    #[test]
    fn test_buffer_into_result() {
        let mut buffer = StreamBuffer::new();
        buffer.push_text("Response text");
        buffer.set_done(true);
        buffer.set_usage(10, 20);

        let result = buffer.into_result();
        assert_eq!(result.content, "Response text");
        assert!(result.done);
        assert_eq!(result.usage.unwrap().prompt_tokens, 10);
    }

    #[test]
    fn test_detect_embedded_tool_call() {
        let content =
            r#"I'll use this tool: {"name": "read_file", "arguments": {"path": "/test"}}"#;
        let calls = detect_embedded_tool_calls(content);

        assert!(calls.is_some());
        let calls = calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "read_file");
    }

    #[test]
    fn test_detect_no_tool_call() {
        let content = "Just some regular text without any tool calls.";
        let calls = detect_embedded_tool_calls(content);
        assert!(calls.is_none());
    }

    #[test]
    fn test_buffer_reset() {
        let mut buffer = StreamBuffer::new();
        buffer.push_text("Some text");
        buffer.set_done(true);

        buffer.reset();
        assert!(buffer.content().is_empty());
        assert!(!buffer.is_done());
    }
}
