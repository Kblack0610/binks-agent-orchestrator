//! Parameter types for Memory MCP tools

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// Session Layer Parameters
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThinkParams {
    #[schemars(description = "The thinking content/observation to record")]
    pub thought: String,

    #[schemars(description = "Confidence level from 0.0 to 1.0 (default: 0.5)")]
    #[serde(default = "default_confidence")]
    pub confidence: f32,

    #[schemars(description = "Optional ID of a previous thought this revises")]
    pub revises: Option<String>,
}

fn default_confidence() -> f32 {
    0.5
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RememberParams {
    #[schemars(description = "The key to store the value under")]
    pub key: String,

    #[schemars(description = "The value to store (string, number, boolean, array, or object)")]
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RecallParams {
    #[schemars(description = "The key to retrieve")]
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CacheToolResultParams {
    #[schemars(description = "Name of the tool that was called")]
    pub tool_name: String,

    #[schemars(description = "Summary of the tool result")]
    pub summary: String,

    #[schemars(description = "Optional TTL in seconds (how long to keep cached)")]
    pub ttl_seconds: Option<u64>,
}

// ============================================================================
// Persistent Layer Parameters
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LearnParams {
    #[schemars(description = "Entity name (e.g., 'project:myapp', 'user:john')")]
    pub entity: String,

    #[schemars(description = "Entity type (e.g., 'project', 'user', 'concept')")]
    pub entity_type: String,

    #[schemars(description = "Facts about the entity as key-value pairs")]
    #[serde(default)]
    pub facts: Vec<FactInput>,

    #[schemars(description = "Relations to other entities")]
    #[serde(default)]
    pub relations: Vec<RelationInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FactInput {
    #[schemars(description = "Fact key/attribute name")]
    pub key: String,

    #[schemars(description = "Fact value")]
    pub value: String,

    #[schemars(description = "Source of fact: 'user', 'inferred', 'learned'")]
    #[serde(default = "default_source")]
    pub source: String,

    #[schemars(description = "Confidence score from 0.0 to 1.0")]
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

fn default_source() -> String {
    "learned".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelationInput {
    #[schemars(description = "Target entity name")]
    pub to_entity: String,

    #[schemars(description = "Relation type (e.g., 'depends_on', 'created_by')")]
    pub relation_type: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct QueryParams {
    #[schemars(description = "Entity name pattern to search (optional, uses LIKE matching)")]
    pub entity_pattern: Option<String>,

    #[schemars(description = "Filter by entity type")]
    pub entity_type: Option<String>,

    #[schemars(description = "Include relations in results")]
    #[serde(default = "default_true")]
    pub include_relations: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ForgetParams {
    #[schemars(description = "Entity name to forget/delete")]
    pub entity: String,
}
