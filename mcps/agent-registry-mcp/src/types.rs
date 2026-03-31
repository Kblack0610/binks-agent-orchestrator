//! Type definitions for agent-registry-mcp

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ============================================================================
// Enums
// ============================================================================

/// Error type for parsing AgentStatus from string
#[derive(Debug, Clone)]
pub struct ParseAgentStatusError(String);

impl fmt::Display for ParseAgentStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid agent status: {}", self.0)
    }
}

impl std::error::Error for ParseAgentStatusError {}

/// Agent status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Active,
    Idle,
    Busy,
    Stale,
    Deregistered,
}

impl AgentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            AgentStatus::Active => "active",
            AgentStatus::Idle => "idle",
            AgentStatus::Busy => "busy",
            AgentStatus::Stale => "stale",
            AgentStatus::Deregistered => "deregistered",
        }
    }
}

impl FromStr for AgentStatus {
    type Err = ParseAgentStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(AgentStatus::Active),
            "idle" => Ok(AgentStatus::Idle),
            "busy" => Ok(AgentStatus::Busy),
            "stale" => Ok(AgentStatus::Stale),
            "deregistered" => Ok(AgentStatus::Deregistered),
            _ => Err(ParseAgentStatusError(s.to_string())),
        }
    }
}

/// Agent type classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    ClaudeCode,
    Binks,
    CustomLlm,
    Unknown,
}

impl AgentType {
    pub fn as_str(&self) -> &str {
        match self {
            AgentType::ClaudeCode => "claude_code",
            AgentType::Binks => "binks",
            AgentType::CustomLlm => "custom_llm",
            AgentType::Unknown => "unknown",
        }
    }
}

/// Error type for parsing AgentType from string
#[derive(Debug, Clone)]
pub struct ParseAgentTypeError(String);

impl fmt::Display for ParseAgentTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid agent type: {}", self.0)
    }
}

impl std::error::Error for ParseAgentTypeError {}

impl FromStr for AgentType {
    type Err = ParseAgentTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "claude_code" => Ok(AgentType::ClaudeCode),
            "binks" => Ok(AgentType::Binks),
            "custom_llm" => Ok(AgentType::CustomLlm),
            "unknown" => Ok(AgentType::Unknown),
            _ => Err(ParseAgentTypeError(s.to_string())),
        }
    }
}

// ============================================================================
// Core Structs
// ============================================================================

/// Agent record in the registry
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentRecord {
    pub agent_id: String,
    pub name: String,
    pub agent_type: String,
    pub pid: Option<i64>,
    pub port: Option<i64>,
    pub working_directory: Option<String>,
    pub active_project: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub status: AgentStatus,
    pub ttl_seconds: i64,
    pub last_heartbeat: String,
    pub registered_at: String,
    pub deregistered_at: Option<String>,
    pub metadata: Option<String>,
}

/// Port claim record
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PortClaim {
    pub port: i64,
    pub agent_id: String,
    pub claimed_at: String,
    pub released_at: Option<String>,
    pub purpose: Option<String>,
}

/// Resource claim record
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceClaim {
    pub id: String,
    pub agent_id: String,
    pub resource_type: String,
    pub resource_identifier: String,
    pub exclusive: bool,
    pub claimed_at: String,
    pub released_at: Option<String>,
    pub purpose: Option<String>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for agent list queries
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentListResponse {
    pub agents: Vec<AgentRecord>,
    pub total: usize,
    pub stale_count: usize,
}

/// Result of a port claim attempt
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PortClaimResult {
    pub success: bool,
    pub port: i64,
    pub agent_id: String,
    pub conflict: Option<PortConflict>,
}

/// Details about a port conflict
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PortConflict {
    pub held_by_agent_id: String,
    pub held_by_name: String,
    pub claimed_at: String,
}

/// Result of a resource claim attempt
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceClaimResult {
    pub success: bool,
    pub claim_id: Option<String>,
    pub conflict: Option<ResourceConflict>,
}

/// Details about a resource conflict
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceConflict {
    pub held_by_agent_id: String,
    pub held_by_name: String,
    pub resource_type: String,
    pub resource_identifier: String,
    pub claimed_at: String,
}

/// Response for who_is_working_on queries
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WhoIsWorkingOnResponse {
    pub resource_identifier: String,
    pub agents: Vec<AgentRecord>,
    pub claims: Vec<ResourceClaim>,
}

/// Response for list_claims queries
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClaimsListResponse {
    pub port_claims: Vec<PortClaim>,
    pub resource_claims: Vec<ResourceClaim>,
}

/// Response for cleanup_stale operations
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CleanupResult {
    pub dry_run: bool,
    pub expired_agents: Vec<AgentRecord>,
    pub released_port_claims: usize,
    pub released_resource_claims: usize,
}
