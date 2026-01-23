//! Memory operation handlers
//!
//! Each handler takes the session/persistent memory and params to perform memory operations.

use chrono::Utc;
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;

use crate::params::*;
use crate::persistent::PersistentMemory;
use crate::session::SessionMemory;
use crate::types::{
    ForgetResponse, LearnResponse, MemoryValue, QueryResponse, RecallResponse, RememberResponse,
    SummarizeResponse, ThinkResponse,
};

// ============================================================================
// Session Layer Handlers
// ============================================================================

pub async fn think(
    session: &SessionMemory,
    params: ThinkParams,
) -> Result<CallToolResult, McpError> {
    let thought = session
        .think(params.thought, params.confidence, params.revises)
        .await;

    let step_number = session.thought_count().await;

    let response = ThinkResponse {
        thought_id: thought.id,
        step_number,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

pub async fn remember(
    session: &SessionMemory,
    params: RememberParams,
) -> Result<CallToolResult, McpError> {
    let memory_value = MemoryValue::from(params.value);
    session.remember(params.key.clone(), memory_value).await;

    let response = RememberResponse {
        key: params.key,
        success: true,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

pub async fn recall(
    session: &SessionMemory,
    params: RecallParams,
) -> Result<CallToolResult, McpError> {
    let value = session.recall(&params.key).await;

    let response = RecallResponse {
        key: params.key,
        found: value.is_some(),
        value,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

pub async fn get_context(session: &SessionMemory) -> Result<CallToolResult, McpError> {
    let context = session.get_full_context().await;

    let json = serde_json::to_string_pretty(&context)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

pub async fn cache_tool_result(
    session: &SessionMemory,
    params: CacheToolResultParams,
) -> Result<CallToolResult, McpError> {
    session
        .cache_tool_result(params.tool_name.clone(), params.summary, params.ttl_seconds)
        .await;

    let response = serde_json::json!({
        "cached": true,
        "tool_name": params.tool_name,
        "ttl_seconds": params.ttl_seconds
    });

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

pub async fn reset_session(session: &SessionMemory) -> Result<CallToolResult, McpError> {
    session.reset().await;

    let response = serde_json::json!({
        "reset": true,
        "new_session_id": session.session_id().await
    });

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

// ============================================================================
// Persistent Layer Handlers
// ============================================================================

pub async fn learn(
    persistent: &PersistentMemory,
    params: LearnParams,
) -> Result<CallToolResult, McpError> {
    // Get or create entity
    let entity = persistent
        .get_or_create_entity(&params.entity, &params.entity_type)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let mut facts_added = 0;
    let mut relations_added = 0;

    // Add facts
    for fact in params.facts {
        persistent
            .add_fact(
                &entity.id,
                &fact.key,
                &fact.value,
                &fact.source,
                fact.confidence,
            )
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        facts_added += 1;
    }

    // Add relations
    for relation in params.relations {
        // Get or create target entity (with unknown type if not exists)
        let target = persistent
            .get_or_create_entity(&relation.to_entity, "unknown")
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        persistent
            .add_relation(&entity.id, &target.id, &relation.relation_type)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        relations_added += 1;
    }

    let response = LearnResponse {
        entity_id: entity.id,
        facts_added,
        relations_added,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

pub async fn query(
    persistent: &PersistentMemory,
    params: QueryParams,
) -> Result<CallToolResult, McpError> {
    // Build search pattern - default to all if no pattern specified
    let pattern = params.entity_pattern.unwrap_or_else(|| "%".to_string());

    // Query entities (returns EntityWithFacts directly)
    let entities_with_facts = persistent
        .query_entities(&pattern)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    // Collect relations if requested
    let mut all_relations = Vec::new();
    if params.include_relations {
        for ewf in &entities_with_facts {
            let relations = persistent
                .get_relations(&ewf.entity.id)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            all_relations.extend(relations);
        }
    }

    let response = QueryResponse {
        entities: entities_with_facts,
        relations: all_relations,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

pub async fn summarize_session(
    session: &SessionMemory,
    persistent: &PersistentMemory,
) -> Result<CallToolResult, McpError> {
    let session_id = session.session_id().await;
    let started_at = session.started_at().await;
    let thought_count = session.thought_count().await;
    let session_end = Utc::now();

    // Get session summary
    let content = session.get_session_summary().await;

    // Save to persistent storage
    let summary = persistent
        .save_summary(&session_id, &content, thought_count, started_at, session_end)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let response = SummarizeResponse {
        summary_id: summary.id,
        content: summary.content,
        thoughts_compressed: thought_count,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

pub async fn forget(
    persistent: &PersistentMemory,
    params: ForgetParams,
) -> Result<CallToolResult, McpError> {
    let (facts_removed, relations_removed) = persistent
        .delete_entity(&params.entity)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let response = ForgetResponse {
        entity_name: params.entity,
        deleted: facts_removed > 0 || relations_removed > 0,
        facts_removed,
        relations_removed,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}
