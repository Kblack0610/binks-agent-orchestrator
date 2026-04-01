//! Parameter definitions for agent-registry-mcp tools

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// Agent Lifecycle
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegisterAgentParams {
    /// Human-readable agent name (e.g. "claude-code-1", "binks-planner")
    pub name: String,
    /// Agent type: "claude_code", "binks", "custom_llm", "unknown"
    #[serde(default)]
    pub agent_type: Option<String>,
    /// OS process ID
    #[serde(default)]
    pub pid: Option<i64>,
    /// Network port if applicable
    #[serde(default)]
    pub port: Option<i64>,
    /// Current working directory
    #[serde(default)]
    pub working_directory: Option<String>,
    /// Repo or project being worked on
    #[serde(default)]
    pub active_project: Option<String>,
    /// Agent capabilities (e.g. ["code_review", "planning", "testing"])
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
    /// Heartbeat TTL in seconds (default 300)
    #[serde(default)]
    pub ttl_seconds: Option<i64>,
    /// Arbitrary JSON metadata
    #[serde(default)]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeregisterAgentParams {
    /// Agent ID (UUID) to deregister
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HeartbeatParams {
    /// Agent ID (UUID)
    pub agent_id: String,
    /// Optionally update status with heartbeat
    #[serde(default)]
    pub status: Option<String>,
    /// Optionally update active project
    #[serde(default)]
    pub active_project: Option<String>,
    /// Optionally update metadata
    #[serde(default)]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateAgentParams {
    /// Agent ID (UUID)
    pub agent_id: String,
    /// New status
    #[serde(default)]
    pub status: Option<String>,
    /// New active project
    #[serde(default)]
    pub active_project: Option<String>,
    /// New working directory
    #[serde(default)]
    pub working_directory: Option<String>,
    /// New capabilities list
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
    /// New metadata
    #[serde(default)]
    pub metadata: Option<String>,
}

// ============================================================================
// Port Management
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClaimPortParams {
    /// Agent ID (UUID) claiming the port
    pub agent_id: String,
    /// Port number to claim
    pub port: i64,
    /// Purpose of the port (e.g. "dev-server", "debug", "api")
    #[serde(default)]
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReleasePortParams {
    /// Agent ID (UUID) releasing the port
    pub agent_id: String,
    /// Port number to release
    pub port: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WhoHasPortParams {
    /// Port number to query
    pub port: i64,
}

// ============================================================================
// Resource Management
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClaimResourceParams {
    /// Agent ID (UUID) claiming the resource
    pub agent_id: String,
    /// Resource type: "directory", "file", "project", "branch", "database"
    pub resource_type: String,
    /// Resource identifier (path, repo URL, branch name, etc.)
    pub resource_identifier: String,
    /// Exclusive lock (default true). False allows shared access.
    #[serde(default)]
    pub exclusive: Option<bool>,
    /// Purpose of the claim
    #[serde(default)]
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseResourceParams {
    /// Resource claim ID (UUID) to release
    pub claim_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseAllResourcesParams {
    /// Agent ID (UUID) whose claims to release
    pub agent_id: String,
}

// ============================================================================
// Queries
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListAgentsParams {
    /// Filter by status (active, idle, busy, stale, deregistered)
    #[serde(default)]
    pub status: Option<String>,
    /// Filter by agent type
    #[serde(default)]
    pub agent_type: Option<String>,
    /// Filter by active project (substring match)
    #[serde(default)]
    pub active_project: Option<String>,
    /// Include stale agents (default false)
    #[serde(default)]
    pub include_stale: Option<bool>,
    /// Max results
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetAgentParams {
    /// Agent ID (UUID)
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WhoIsWorkingOnParams {
    /// Resource identifier to search for (repo name, directory path, etc.)
    pub resource_identifier: String,
    /// Optionally narrow search by resource type
    #[serde(default)]
    pub resource_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListClaimsParams {
    /// Filter by agent ID
    #[serde(default)]
    pub agent_id: Option<String>,
    /// Filter by resource type
    #[serde(default)]
    pub resource_type: Option<String>,
    /// Only show active claims (default true)
    #[serde(default)]
    pub active_only: Option<bool>,
    /// Max results
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CleanupStaleParams {
    /// Preview what would be cleaned without making changes (default false)
    #[serde(default)]
    pub dry_run: Option<bool>,
}
