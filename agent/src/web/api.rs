//! REST API handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use super::state::AppState;
use crate::db::{
    conversations::{Conversation, CreateConversation, UpdateConversation},
    messages::Message,
};

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    fn new(msg: impl Into<String>) -> Self {
        Self { error: msg.into() }
    }
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub database: bool,
    pub mcp_available: bool,
    pub ollama_url: String,
    pub model: String,
}

/// Health check endpoint
pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        database: true, // If we got here, DB is working
        mcp_available: state.mcp_pool.is_some(),
        ollama_url: state.ollama_url.clone(),
        model: state.model.clone(),
    })
}

/// Pagination query parameters
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// List conversations response
#[derive(Debug, Serialize)]
pub struct ConversationsListResponse {
    pub conversations: Vec<Conversation>,
    pub total: usize,
}

/// List all conversations
pub async fn list_conversations(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ConversationsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.db.list_conversations(params.limit, params.offset) {
        Ok(conversations) => {
            let total = conversations.len();
            Ok(Json(ConversationsListResponse {
                conversations,
                total,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to list conversations: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

/// Create conversation request
#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
    pub system_prompt: Option<String>,
}

/// Create a new conversation
pub async fn create_conversation(
    State(state): State<AppState>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<(StatusCode, Json<Conversation>), (StatusCode, Json<ErrorResponse>)> {
    let params = CreateConversation {
        title: req.title,
        system_prompt: req.system_prompt.or_else(|| state.system_prompt.clone()),
        metadata: None,
    };

    match state.db.create_conversation(params) {
        Ok(conversation) => Ok((StatusCode::CREATED, Json(conversation))),
        Err(e) => {
            tracing::error!("Failed to create conversation: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

/// Conversation with messages response
#[derive(Debug, Serialize)]
pub struct ConversationWithMessages {
    #[serde(flatten)]
    pub conversation: Conversation,
    pub messages: Vec<Message>,
}

/// Get a conversation by ID (with messages)
pub async fn get_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ConversationWithMessages>, (StatusCode, Json<ErrorResponse>)> {
    match state.db.get_conversation(&id) {
        Ok(Some(conversation)) => {
            let messages = state.db.get_messages(&id).unwrap_or_default();
            Ok(Json(ConversationWithMessages {
                conversation,
                messages,
            }))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("Conversation not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get conversation: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

/// Update conversation request
#[derive(Debug, Deserialize)]
pub struct UpdateConversationRequest {
    pub title: Option<String>,
    pub system_prompt: Option<String>,
}

/// Update a conversation
pub async fn update_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateConversationRequest>,
) -> Result<Json<Conversation>, (StatusCode, Json<ErrorResponse>)> {
    let params = UpdateConversation {
        title: req.title,
        system_prompt: req.system_prompt,
        metadata: None,
    };

    match state.db.update_conversation(&id, params) {
        Ok(Some(conversation)) => Ok(Json(conversation)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("Conversation not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to update conversation: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

/// Delete a conversation
pub async fn delete_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.db.delete_conversation(&id) {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("Conversation not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete conversation: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

/// Get messages for a conversation
pub async fn get_messages(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Message>>, (StatusCode, Json<ErrorResponse>)> {
    // First check if conversation exists
    match state.db.get_conversation(&id) {
        Ok(Some(_)) => match state.db.get_messages(&id) {
            Ok(messages) => Ok(Json(messages)),
            Err(e) => {
                tracing::error!("Failed to get messages: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(e.to_string())),
                ))
            }
        },
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("Conversation not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to check conversation: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}

/// Tool info for API response
#[derive(Debug, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub server: String,
    pub description: Option<String>,
}

/// List available tools
pub async fn list_tools(
    State(state): State<AppState>,
) -> Result<Json<Vec<ToolInfo>>, (StatusCode, Json<ErrorResponse>)> {
    match &state.mcp_pool {
        Some(pool) => {
            let mut pool = pool.lock().await;
            match pool.list_all_tools().await {
                Ok(tools) => {
                    let tool_infos: Vec<ToolInfo> = tools
                        .into_iter()
                        .map(|t| ToolInfo {
                            name: t.name,
                            server: t.server,
                            description: t.description,
                        })
                        .collect();
                    Ok(Json(tool_infos))
                }
                Err(e) => {
                    tracing::error!("Failed to list tools: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(e.to_string())),
                    ))
                }
            }
        }
        None => Ok(Json(vec![])), // No tools available
    }
}

/// Models list response
#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub models: Vec<crate::llm::ModelInfo>,
    pub current: String,
}

/// List available Ollama models
pub async fn list_models(
    State(state): State<AppState>,
) -> Result<Json<ModelsResponse>, (StatusCode, Json<ErrorResponse>)> {
    match crate::llm::list_models(&state.ollama_url).await {
        Ok(models) => Ok(Json(ModelsResponse {
            models,
            current: state.model.clone(),
        })),
        Err(e) => {
            tracing::error!("Failed to list models: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string())),
            ))
        }
    }
}
