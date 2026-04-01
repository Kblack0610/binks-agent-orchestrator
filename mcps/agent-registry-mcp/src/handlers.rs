//! Handler implementations for agent-registry-mcp tools
//!
//! Each handler converts MCP params to repository types, calls the repository,
//! and converts results to CallToolResult with proper error handling.

use mcp_common::{internal_error, invalid_params, json_success, CallToolResult, McpError};
use serde_json::json;

use crate::params::*;
use crate::repository::{AgentFilter, AgentRegistryRepository, ClaimFilter, NewAgent};
use crate::types::{AgentListResponse, ClaimsListResponse};

// ============================================================================
// Agent Lifecycle
// ============================================================================

pub async fn register_agent(
    repo: &AgentRegistryRepository,
    params: RegisterAgentParams,
) -> Result<CallToolResult, McpError> {
    let new_agent = NewAgent {
        name: params.name,
        agent_type: params.agent_type.unwrap_or_else(|| "unknown".to_string()),
        pid: params.pid,
        port: params.port,
        working_directory: params.working_directory,
        active_project: params.active_project,
        capabilities: params.capabilities,
        ttl_seconds: params.ttl_seconds.unwrap_or(300),
        metadata: params.metadata,
    };

    let agent = repo
        .register_agent(new_agent)
        .map_err(|e| internal_error(format!("Failed to register agent: {}", e)))?;

    json_success(&agent)
}

pub async fn deregister_agent(
    repo: &AgentRegistryRepository,
    params: DeregisterAgentParams,
) -> Result<CallToolResult, McpError> {
    repo.deregister_agent(&params.agent_id)
        .map_err(|e| internal_error(format!("Failed to deregister agent: {}", e)))?;

    json_success(&json!({
        "success": true,
        "agent_id": params.agent_id,
        "message": format!("Agent {} deregistered and all claims released", params.agent_id)
    }))
}

pub async fn heartbeat(
    repo: &AgentRegistryRepository,
    params: HeartbeatParams,
) -> Result<CallToolResult, McpError> {
    let agent = repo
        .heartbeat(
            &params.agent_id,
            params.status.as_deref(),
            params.active_project.as_deref(),
            params.metadata.as_deref(),
        )
        .map_err(|e| internal_error(format!("Failed to heartbeat: {}", e)))?;

    json_success(&agent)
}

pub async fn update_agent(
    repo: &AgentRegistryRepository,
    params: UpdateAgentParams,
) -> Result<CallToolResult, McpError> {
    repo.update_agent_fields(
        &params.agent_id,
        params.status.as_deref(),
        params.active_project.as_deref(),
        params.working_directory.as_deref(),
        params.capabilities.as_deref(),
        params.metadata.as_deref(),
    )
    .map_err(|e| internal_error(format!("Failed to update agent: {}", e)))?;

    // Fetch updated agent
    let agent = repo
        .get_agent(&params.agent_id)
        .map_err(|e| internal_error(format!("Failed to get updated agent: {}", e)))?
        .ok_or_else(|| invalid_params(format!("Agent not found: {}", params.agent_id)))?;

    json_success(&agent)
}

// ============================================================================
// Port Management
// ============================================================================

pub async fn claim_port(
    repo: &AgentRegistryRepository,
    params: ClaimPortParams,
) -> Result<CallToolResult, McpError> {
    let result = repo
        .claim_port(&params.agent_id, params.port, params.purpose.as_deref())
        .map_err(|e| internal_error(format!("Failed to claim port: {}", e)))?;

    json_success(&result)
}

pub async fn release_port(
    repo: &AgentRegistryRepository,
    params: ReleasePortParams,
) -> Result<CallToolResult, McpError> {
    repo.release_port(&params.agent_id, params.port)
        .map_err(|e| internal_error(format!("Failed to release port: {}", e)))?;

    json_success(&json!({
        "success": true,
        "agent_id": params.agent_id,
        "port": params.port,
        "message": format!("Port {} released", params.port)
    }))
}

pub async fn who_has_port(
    repo: &AgentRegistryRepository,
    params: WhoHasPortParams,
) -> Result<CallToolResult, McpError> {
    let result = repo
        .who_has_port(params.port)
        .map_err(|e| internal_error(format!("Failed to query port: {}", e)))?;

    match result {
        Some((claim, agent)) => json_success(&json!({
            "port": params.port,
            "claimed": true,
            "claim": claim,
            "agent": agent,
        })),
        None => json_success(&json!({
            "port": params.port,
            "claimed": false,
            "message": format!("Port {} is not claimed by any active agent", params.port)
        })),
    }
}

// ============================================================================
// Resource Management
// ============================================================================

pub async fn claim_resource(
    repo: &AgentRegistryRepository,
    params: ClaimResourceParams,
) -> Result<CallToolResult, McpError> {
    let exclusive = params.exclusive.unwrap_or(true);

    let result = repo
        .claim_resource(
            &params.agent_id,
            &params.resource_type,
            &params.resource_identifier,
            exclusive,
            params.purpose.as_deref(),
        )
        .map_err(|e| internal_error(format!("Failed to claim resource: {}", e)))?;

    json_success(&result)
}

pub async fn release_resource(
    repo: &AgentRegistryRepository,
    params: ReleaseResourceParams,
) -> Result<CallToolResult, McpError> {
    repo.release_resource(&params.claim_id)
        .map_err(|e| internal_error(format!("Failed to release resource: {}", e)))?;

    json_success(&json!({
        "success": true,
        "claim_id": params.claim_id,
        "message": format!("Resource claim {} released", params.claim_id)
    }))
}

pub async fn release_all_resources(
    repo: &AgentRegistryRepository,
    params: ReleaseAllResourcesParams,
) -> Result<CallToolResult, McpError> {
    let (ports, resources) = repo
        .release_all_for_agent(&params.agent_id)
        .map_err(|e| internal_error(format!("Failed to release resources: {}", e)))?;

    json_success(&json!({
        "success": true,
        "agent_id": params.agent_id,
        "released_port_claims": ports,
        "released_resource_claims": resources,
        "message": format!("Released {} port claims and {} resource claims", ports, resources)
    }))
}

// ============================================================================
// Queries
// ============================================================================

pub async fn list_agents(
    repo: &AgentRegistryRepository,
    params: ListAgentsParams,
) -> Result<CallToolResult, McpError> {
    let filter = AgentFilter {
        status: params.status,
        agent_type: params.agent_type,
        active_project: params.active_project,
        include_stale: params.include_stale.unwrap_or(false),
        limit: params.limit,
    };

    let agents = repo
        .list_agents(filter)
        .map_err(|e| internal_error(format!("Failed to list agents: {}", e)))?;

    let stale_count = agents
        .iter()
        .filter(|a| a.status == crate::types::AgentStatus::Stale)
        .count();

    let response = AgentListResponse {
        total: agents.len(),
        stale_count,
        agents,
    };

    json_success(&response)
}

pub async fn get_agent(
    repo: &AgentRegistryRepository,
    params: GetAgentParams,
) -> Result<CallToolResult, McpError> {
    let agent = repo
        .get_agent(&params.agent_id)
        .map_err(|e| internal_error(format!("Failed to get agent: {}", e)))?;

    match agent {
        Some(agent) => json_success(&agent),
        None => Err(invalid_params(format!(
            "Agent not found: {}",
            params.agent_id
        ))),
    }
}

pub async fn who_is_working_on(
    repo: &AgentRegistryRepository,
    params: WhoIsWorkingOnParams,
) -> Result<CallToolResult, McpError> {
    let result = repo
        .who_is_working_on(&params.resource_identifier, params.resource_type.as_deref())
        .map_err(|e| internal_error(format!("Failed to query: {}", e)))?;

    json_success(&result)
}

pub async fn list_claims(
    repo: &AgentRegistryRepository,
    params: ListClaimsParams,
) -> Result<CallToolResult, McpError> {
    let filter = ClaimFilter {
        agent_id: params.agent_id,
        resource_type: params.resource_type,
        active_only: params.active_only.unwrap_or(true),
        limit: params.limit,
    };

    let (port_claims, resource_claims) = repo
        .list_claims(filter)
        .map_err(|e| internal_error(format!("Failed to list claims: {}", e)))?;

    let response = ClaimsListResponse {
        port_claims,
        resource_claims,
    };

    json_success(&response)
}

pub async fn cleanup_stale(
    repo: &AgentRegistryRepository,
    params: CleanupStaleParams,
) -> Result<CallToolResult, McpError> {
    let dry_run = params.dry_run.unwrap_or(false);

    let result = repo
        .cleanup_stale(dry_run)
        .map_err(|e| internal_error(format!("Failed to cleanup stale agents: {}", e)))?;

    json_success(&result)
}
