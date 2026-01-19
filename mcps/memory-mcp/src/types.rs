//! Type definitions for memory storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Session Layer Types
// ============================================================================

/// A single thinking step in the reasoning chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    /// Unique ID for this thought
    pub id: String,
    /// The thinking content
    pub content: String,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Timestamp when recorded
    pub timestamp: DateTime<Utc>,
    /// Optional reference to a thought this revises
    pub revises: Option<String>,
}

/// Value that can be stored in working memory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MemoryValue {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<MemoryValue>),
    Object(std::collections::HashMap<String, MemoryValue>),
    Null,
}

impl From<serde_json::Value> for MemoryValue {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => MemoryValue::Null,
            serde_json::Value::Bool(b) => MemoryValue::Bool(b),
            serde_json::Value::Number(n) => MemoryValue::Number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(s) => MemoryValue::String(s),
            serde_json::Value::Array(arr) => {
                MemoryValue::Array(arr.into_iter().map(MemoryValue::from).collect())
            }
            serde_json::Value::Object(obj) => {
                MemoryValue::Object(obj.into_iter().map(|(k, v)| (k, MemoryValue::from(v))).collect())
            }
        }
    }
}

/// A cached tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedToolResult {
    /// Tool name that was called
    pub tool_name: String,
    /// Summary of the result
    pub summary: String,
    /// When the tool was called
    pub timestamp: DateTime<Utc>,
    /// Time-to-live (how long to keep this cached)
    pub ttl_seconds: Option<u64>,
}

// ============================================================================
// Persistent Layer Types
// ============================================================================

/// An entity in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique ID
    pub id: String,
    /// Entity name (e.g., "project:myapp", "user:john")
    pub name: String,
    /// Entity type (e.g., "project", "user", "concept")
    pub entity_type: String,
    /// When created
    pub created_at: DateTime<Utc>,
    /// When last updated
    pub updated_at: DateTime<Utc>,
}

/// A fact about an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    /// Unique ID
    pub id: String,
    /// Entity this fact belongs to
    pub entity_id: String,
    /// The key (attribute name)
    pub key: String,
    /// The value
    pub value: String,
    /// Source of this fact (e.g., "user", "inferred", "learned")
    pub source: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// When this fact was recorded
    pub recorded_at: DateTime<Utc>,
}

/// A relation between two entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    /// Unique ID
    pub id: String,
    /// Source entity ID
    pub from_entity_id: String,
    /// Target entity ID
    pub to_entity_id: String,
    /// Relation type (e.g., "depends_on", "created_by", "related_to")
    pub relation_type: String,
    /// When this relation was recorded
    pub recorded_at: DateTime<Utc>,
}

/// A compressed session summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    /// Unique ID
    pub id: String,
    /// Session identifier
    pub session_id: String,
    /// Compressed summary text
    pub content: String,
    /// Number of thoughts that were compressed
    pub thought_count: usize,
    /// When the session started
    pub session_start: DateTime<Utc>,
    /// When the session ended
    pub session_end: DateTime<Utc>,
    /// When this summary was created
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for think operation
#[derive(Debug, Serialize, Deserialize)]
pub struct ThinkResponse {
    pub thought_id: String,
    pub step_number: usize,
}

/// Response for remember operation
#[derive(Debug, Serialize, Deserialize)]
pub struct RememberResponse {
    pub key: String,
    pub success: bool,
}

/// Response for recall operation
#[derive(Debug, Serialize, Deserialize)]
pub struct RecallResponse {
    pub key: String,
    pub value: Option<MemoryValue>,
    pub found: bool,
}

/// Full session context
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionContext {
    pub thoughts: Vec<Thought>,
    pub context: std::collections::HashMap<String, MemoryValue>,
    pub tool_results: Vec<CachedToolResult>,
    pub session_id: String,
    pub started_at: DateTime<Utc>,
}

/// Response for learn operation
#[derive(Debug, Serialize, Deserialize)]
pub struct LearnResponse {
    pub entity_id: String,
    pub facts_added: usize,
    pub relations_added: usize,
}

/// Response for query operation
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    pub entities: Vec<EntityWithFacts>,
    pub relations: Vec<Relation>,
}

/// Entity with its associated facts
#[derive(Debug, Serialize, Deserialize)]
pub struct EntityWithFacts {
    pub entity: Entity,
    pub facts: Vec<Fact>,
}

/// Response for summarize session
#[derive(Debug, Serialize, Deserialize)]
pub struct SummarizeResponse {
    pub summary_id: String,
    pub content: String,
    pub thoughts_compressed: usize,
}

/// Response for forget operation
#[derive(Debug, Serialize, Deserialize)]
pub struct ForgetResponse {
    pub entity_name: String,
    pub deleted: bool,
    pub facts_removed: usize,
    pub relations_removed: usize,
}
