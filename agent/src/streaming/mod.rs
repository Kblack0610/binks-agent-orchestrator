//! Streaming support for LLM responses
//!
//! This module provides streaming capabilities for LLM responses with
//! proper handling of tool calls. Text tokens are streamed immediately
//! while tool call detection happens at stream completion.

mod buffer;

pub use buffer::{StreamBuffer, StreamChunk};

use crate::agent::events::AgentEvent;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// ============================================================================
// Streaming Response Types (Ollama format)
// ============================================================================

/// A streaming response chunk from Ollama
#[derive(Debug, Clone, Deserialize)]
pub struct OllamaStreamChunk {
    /// The model name
    pub model: Option<String>,
    /// The message content
    pub message: Option<StreamMessage>,
    /// Whether this is the final chunk
    pub done: bool,
    /// Reason for completion (if done)
    pub done_reason: Option<String>,
    /// Total duration in nanoseconds (only on final chunk)
    pub total_duration: Option<u64>,
    /// Token count for prompt (only on final chunk)
    pub prompt_eval_count: Option<u32>,
    /// Token count for response (only on final chunk)
    pub eval_count: Option<u32>,
}

/// Message content in a stream chunk
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StreamMessage {
    /// Role (usually "assistant")
    pub role: Option<String>,
    /// Text content
    pub content: Option<String>,
    /// Tool calls (usually only on final chunk)
    pub tool_calls: Option<Vec<StreamToolCall>>,
}

/// A tool call from the stream
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StreamToolCall {
    /// The function to call
    pub function: StreamFunction,
}

/// Function details in a tool call
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StreamFunction {
    /// Function name
    pub name: String,
    /// Arguments as JSON
    pub arguments: serde_json::Value,
}

// ============================================================================
// Streaming Trait
// ============================================================================

/// Trait for streaming LLM responses
#[async_trait]
pub trait LlmStreaming: Send + Sync {
    /// Stream a chat response, sending events through the channel
    ///
    /// Returns the final complete response with any tool calls
    async fn stream_chat(
        &self,
        messages: &[serde_json::Value],
        tools: Option<&[serde_json::Value]>,
        event_tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<StreamingResult>;
}

/// Result from a streaming chat completion
#[derive(Debug, Clone)]
pub struct StreamingResult {
    /// The complete text response (accumulated from all chunks)
    pub content: String,
    /// Any tool calls detected
    pub tool_calls: Vec<StreamToolCall>,
    /// Whether the stream completed successfully
    pub done: bool,
    /// Token usage stats
    pub usage: Option<StreamUsage>,
}

/// Token usage statistics
#[derive(Debug, Clone)]
pub struct StreamUsage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,
    /// Tokens in the response
    pub completion_tokens: u32,
}

// ============================================================================
// Ollama Streaming Implementation
// ============================================================================

/// Ollama streaming client
pub struct OllamaStreamer {
    /// Base URL for Ollama API
    base_url: String,
    /// Model to use
    model: String,
    /// HTTP client
    client: reqwest::Client,
}

impl OllamaStreamer {
    /// Create a new Ollama streamer
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Parse a streaming response line
    pub fn parse_chunk(line: &str) -> Result<Option<OllamaStreamChunk>> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }

        let chunk: OllamaStreamChunk = serde_json::from_str(line)?;
        Ok(Some(chunk))
    }
}

#[async_trait]
impl LlmStreaming for OllamaStreamer {
    async fn stream_chat(
        &self,
        messages: &[serde_json::Value],
        tools: Option<&[serde_json::Value]>,
        event_tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<StreamingResult> {
        use futures_util::StreamExt;

        // Build request body
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(tools) = tools {
            body["tools"] = serde_json::Value::Array(tools.to_vec());
        }

        // Make streaming request
        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama streaming request failed: {} - {}", status, text);
        }

        // Process stream
        let mut buffer = StreamBuffer::new();
        let mut bytes_stream = response.bytes_stream();
        let mut line_buffer = String::new();

        while let Some(chunk_result) = bytes_stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);

            // Ollama sends newline-delimited JSON
            line_buffer.push_str(&text);

            // Process complete lines
            while let Some(newline_pos) = line_buffer.find('\n') {
                let line = line_buffer[..newline_pos].to_string();
                line_buffer = line_buffer[newline_pos + 1..].to_string();

                if let Some(stream_chunk) = Self::parse_chunk(&line)? {
                    // Process the chunk
                    if let Some(msg) = &stream_chunk.message {
                        // Stream text content immediately
                        if let Some(content) = &msg.content {
                            if !content.is_empty() {
                                buffer.push_text(content);
                                // Send token event
                                let _ = event_tx.send(AgentEvent::Token {
                                    content: content.clone(),
                                });
                            }
                        }

                        // Capture tool calls (usually on final chunk)
                        if let Some(tool_calls) = &msg.tool_calls {
                            buffer.set_tool_calls(tool_calls.clone());
                        }
                    }

                    // Check for completion
                    if stream_chunk.done {
                        buffer.set_done(true);
                        if let (Some(prompt), Some(completion)) =
                            (stream_chunk.prompt_eval_count, stream_chunk.eval_count)
                        {
                            buffer.set_usage(prompt, completion);
                        }
                    }
                }
            }
        }

        // Build result
        let result = buffer.into_result();
        Ok(result)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chunk_empty() {
        let result = OllamaStreamer::parse_chunk("");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_chunk_text() {
        let line = r#"{"message":{"role":"assistant","content":"Hello"},"done":false}"#;
        let result = OllamaStreamer::parse_chunk(line);
        assert!(result.is_ok());
        let chunk = result.unwrap().unwrap();
        assert!(!chunk.done);
        assert_eq!(
            chunk.message.unwrap().content.unwrap(),
            "Hello"
        );
    }

    #[test]
    fn test_parse_chunk_done() {
        let line = r#"{"message":{"role":"assistant","content":""},"done":true,"done_reason":"stop","total_duration":1234567890,"prompt_eval_count":10,"eval_count":20}"#;
        let result = OllamaStreamer::parse_chunk(line);
        assert!(result.is_ok());
        let chunk = result.unwrap().unwrap();
        assert!(chunk.done);
        assert_eq!(chunk.prompt_eval_count.unwrap(), 10);
        assert_eq!(chunk.eval_count.unwrap(), 20);
    }
}
