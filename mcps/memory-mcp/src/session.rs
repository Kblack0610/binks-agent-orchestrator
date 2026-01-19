//! Session memory layer - in-memory, ephemeral storage

use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::types::{CachedToolResult, MemoryValue, SessionContext, Thought};

/// Session memory - ephemeral, in-memory storage for the current session
#[derive(Clone)]
pub struct SessionMemory {
    inner: Arc<RwLock<SessionMemoryInner>>,
}

struct SessionMemoryInner {
    /// Current session ID
    session_id: String,
    /// Session start time
    started_at: chrono::DateTime<Utc>,
    /// Reasoning chain
    thoughts: Vec<Thought>,
    /// Working memory (key-value store)
    context: HashMap<String, MemoryValue>,
    /// Cached tool results
    tool_results: Vec<CachedToolResult>,
}

impl SessionMemory {
    /// Create a new session memory
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(SessionMemoryInner {
                session_id: Uuid::new_v4().to_string(),
                started_at: Utc::now(),
                thoughts: Vec::new(),
                context: HashMap::new(),
                tool_results: Vec::new(),
            })),
        }
    }

    /// Get the current session ID
    pub async fn session_id(&self) -> String {
        self.inner.read().await.session_id.clone()
    }

    /// Get the session start time
    pub async fn started_at(&self) -> chrono::DateTime<Utc> {
        self.inner.read().await.started_at
    }

    // ========================================================================
    // Thinking Operations
    // ========================================================================

    /// Record a thinking step
    pub async fn think(&self, content: String, confidence: f32, revises: Option<String>) -> Thought {
        let mut inner = self.inner.write().await;

        let thought = Thought {
            id: Uuid::new_v4().to_string(),
            content,
            confidence: confidence.clamp(0.0, 1.0),
            timestamp: Utc::now(),
            revises,
        };

        inner.thoughts.push(thought.clone());
        thought
    }

    /// Get all thoughts in the reasoning chain
    pub async fn get_thoughts(&self) -> Vec<Thought> {
        self.inner.read().await.thoughts.clone()
    }

    /// Get the number of thoughts
    pub async fn thought_count(&self) -> usize {
        self.inner.read().await.thoughts.len()
    }

    /// Clear the reasoning chain
    pub async fn clear_thoughts(&self) {
        self.inner.write().await.thoughts.clear();
    }

    // ========================================================================
    // Context Operations (Working Memory)
    // ========================================================================

    /// Store a value in working memory
    pub async fn remember(&self, key: String, value: MemoryValue) {
        self.inner.write().await.context.insert(key, value);
    }

    /// Recall a value from working memory
    pub async fn recall(&self, key: &str) -> Option<MemoryValue> {
        self.inner.read().await.context.get(key).cloned()
    }

    /// Remove a value from working memory
    pub async fn forget_context(&self, key: &str) -> bool {
        self.inner.write().await.context.remove(key).is_some()
    }

    /// Get all context keys
    pub async fn context_keys(&self) -> Vec<String> {
        self.inner.read().await.context.keys().cloned().collect()
    }

    /// Get the full context
    pub async fn get_context(&self) -> HashMap<String, MemoryValue> {
        self.inner.read().await.context.clone()
    }

    /// Clear the context
    pub async fn clear_context(&self) {
        self.inner.write().await.context.clear();
    }

    // ========================================================================
    // Tool Result Caching
    // ========================================================================

    /// Cache a tool result
    pub async fn cache_tool_result(&self, tool_name: String, summary: String, ttl_seconds: Option<u64>) {
        let result = CachedToolResult {
            tool_name,
            summary,
            timestamp: Utc::now(),
            ttl_seconds,
        };

        let mut inner = self.inner.write().await;
        inner.tool_results.push(result);

        // Clean up expired results
        let now = Utc::now();
        inner.tool_results.retain(|r| {
            if let Some(ttl) = r.ttl_seconds {
                let expires_at = r.timestamp + chrono::Duration::seconds(ttl as i64);
                expires_at > now
            } else {
                true
            }
        });
    }

    /// Get cached tool results
    pub async fn get_tool_results(&self) -> Vec<CachedToolResult> {
        self.inner.read().await.tool_results.clone()
    }

    /// Clear tool result cache
    pub async fn clear_tool_results(&self) {
        self.inner.write().await.tool_results.clear();
    }

    // ========================================================================
    // Full Session Operations
    // ========================================================================

    /// Get the full session context
    pub async fn get_full_context(&self) -> SessionContext {
        let inner = self.inner.read().await;
        SessionContext {
            thoughts: inner.thoughts.clone(),
            context: inner.context.clone(),
            tool_results: inner.tool_results.clone(),
            session_id: inner.session_id.clone(),
            started_at: inner.started_at,
        }
    }

    /// Reset the session (start fresh)
    pub async fn reset(&self) {
        let mut inner = self.inner.write().await;
        inner.session_id = Uuid::new_v4().to_string();
        inner.started_at = Utc::now();
        inner.thoughts.clear();
        inner.context.clear();
        inner.tool_results.clear();
    }

    /// Get a summary of the session for persistence
    pub async fn get_session_summary(&self) -> String {
        let inner = self.inner.read().await;

        if inner.thoughts.is_empty() {
            return "Empty session - no thoughts recorded.".to_string();
        }

        let mut summary = String::new();

        // Collect high-confidence thoughts
        let key_thoughts: Vec<&Thought> = inner.thoughts
            .iter()
            .filter(|t| t.confidence >= 0.7)
            .collect();

        if !key_thoughts.is_empty() {
            summary.push_str("Key insights:\n");
            for t in key_thoughts {
                summary.push_str(&format!("- {} (confidence: {:.0}%)\n", t.content, t.confidence * 100.0));
            }
        }

        // Note any revisions
        let revisions: Vec<&Thought> = inner.thoughts
            .iter()
            .filter(|t| t.revises.is_some())
            .collect();

        if !revisions.is_empty() {
            summary.push_str("\nRevised thinking:\n");
            for t in revisions {
                summary.push_str(&format!("- {}\n", t.content));
            }
        }

        // Add context summary if non-empty
        if !inner.context.is_empty() {
            summary.push_str(&format!("\nContext keys: {}\n", inner.context.keys().cloned().collect::<Vec<_>>().join(", ")));
        }

        summary
    }
}

impl Default for SessionMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_think_and_recall() {
        let memory = SessionMemory::new();

        let thought = memory.think("First observation".to_string(), 0.8, None).await;
        assert_eq!(thought.content, "First observation");
        assert!(thought.confidence - 0.8 < 0.001);

        let thoughts = memory.get_thoughts().await;
        assert_eq!(thoughts.len(), 1);
    }

    #[tokio::test]
    async fn test_context_operations() {
        let memory = SessionMemory::new();

        memory.remember("key1".to_string(), MemoryValue::String("value1".to_string())).await;

        let value = memory.recall("key1").await;
        assert!(matches!(value, Some(MemoryValue::String(s)) if s == "value1"));

        let missing = memory.recall("nonexistent").await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_reset() {
        let memory = SessionMemory::new();

        memory.think("Test".to_string(), 0.5, None).await;
        memory.remember("key".to_string(), MemoryValue::Bool(true)).await;

        assert_eq!(memory.thought_count().await, 1);

        memory.reset().await;

        assert_eq!(memory.thought_count().await, 0);
        assert!(memory.recall("key").await.is_none());
    }
}
