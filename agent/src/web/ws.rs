//! WebSocket handler for real-time chat

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::state::AppState;
use crate::agent::Agent;
use crate::db::messages::{CreateMessage, MessageRole, ToolCall, ToolResult};

/// Incoming WebSocket message from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Send a chat message
    #[serde(rename = "message")]
    Message {
        content: String,
        /// Optional list of servers to filter tools
        servers: Option<Vec<String>>,
        /// Optional model override for this message
        model: Option<String>,
    },
    /// Cancel current operation
    #[serde(rename = "cancel")]
    Cancel,
    /// Ping for keepalive
    #[serde(rename = "ping")]
    Ping,
}

/// Outgoing WebSocket message to client
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Connection established
    #[serde(rename = "connected")]
    Connected { conversation_id: String },
    /// Streaming token
    #[serde(rename = "token")]
    Token { content: String },
    /// Tool call started
    #[serde(rename = "tool_start")]
    ToolStart { name: String, arguments: serde_json::Value },
    /// Tool call completed
    #[serde(rename = "tool_result")]
    ToolResult {
        name: String,
        result: String,
        is_error: bool,
    },
    /// Complete assistant message
    #[serde(rename = "message")]
    Message {
        id: String,
        content: String,
        tool_calls: Option<Vec<ToolCall>>,
        tool_results: Option<Vec<ToolResult>>,
    },
    /// Error occurred
    #[serde(rename = "error")]
    Error { message: String },
    /// Pong response
    #[serde(rename = "pong")]
    Pong,
}

/// WebSocket handler
pub async fn chat_handler(
    ws: WebSocketUpgrade,
    Path(conversation_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, conversation_id, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, conversation_id: String, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Verify conversation exists
    if state.db.get_conversation(&conversation_id).ok().flatten().is_none() {
        let _ = sender
            .send(Message::Text(
                serde_json::to_string(&ServerMessage::Error {
                    message: "Conversation not found".to_string(),
                })
                .unwrap().into(),
            ))
            .await;
        return;
    }

    // Send connected message
    let connected_msg = ServerMessage::Connected {
        conversation_id: conversation_id.clone(),
    };
    if sender
        .send(Message::Text(serde_json::to_string(&connected_msg).unwrap().into()))
        .await
        .is_err()
    {
        return;
    }

    // Create agent for this session
    let agent = match create_agent(&state).await {
        Ok(agent) => Arc::new(Mutex::new(agent)),
        Err(e) => {
            let _ = sender
                .send(Message::Text(
                    serde_json::to_string(&ServerMessage::Error {
                        message: format!("Failed to create agent: {}", e),
                    })
                    .unwrap().into(),
                ))
                .await;
            return;
        }
    };

    // Load conversation history into agent
    if let Ok(messages) = state.db.get_messages(&conversation_id) {
        let history: Vec<crate::agent::DirectMessage> = messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::System => "system",
                    MessageRole::Tool => "tool",
                };
                crate::agent::DirectMessage::new(role, &m.content)
            })
            .collect();
        agent.lock().await.set_history(history);
    }

    let sender = Arc::new(Mutex::new(sender));

    // Process incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    match client_msg {
                        ClientMessage::Message { content, servers, model } => {
                            handle_chat_message(
                                &content,
                                servers,
                                model,
                                &conversation_id,
                                &state,
                                &agent,
                                &sender,
                            )
                            .await;
                        }
                        ClientMessage::Cancel => {
                            // TODO: Implement cancellation
                            tracing::info!("Cancel requested for {}", conversation_id);
                        }
                        ClientMessage::Ping => {
                            let pong = ServerMessage::Pong;
                            let _ = sender
                                .lock()
                                .await
                                .send(Message::Text(serde_json::to_string(&pong).unwrap().into()))
                                .await;
                        }
                    }
                }
            }
            Message::Close(_) => {
                tracing::info!("WebSocket closed for conversation {}", conversation_id);
                break;
            }
            _ => {}
        }
    }
}

/// Create an agent instance using a fresh MCP pool
async fn create_agent(state: &AppState) -> anyhow::Result<Agent> {
    // Load a fresh MCP pool for this agent instance
    let pool = crate::mcp::McpClientPool::load()?
        .ok_or_else(|| anyhow::anyhow!("No .mcp.json found - agent needs MCP tools"))?;

    let mut agent = Agent::new(&state.ollama_url, &state.model, pool);

    if let Some(ref prompt) = state.system_prompt {
        agent = agent.with_system_prompt(prompt);
    }

    Ok(agent)
}

/// Handle a chat message
async fn handle_chat_message(
    content: &str,
    servers: Option<Vec<String>>,
    model: Option<String>,
    conversation_id: &str,
    state: &AppState,
    agent: &Arc<Mutex<Agent>>,
    sender: &Arc<Mutex<futures_util::stream::SplitSink<WebSocket, Message>>>,
) {
    // Apply model override if provided
    if let Some(ref model_name) = model {
        agent.lock().await.set_model(model_name);
        tracing::info!("Model override applied: {}", model_name);
    }

    // Save user message to database
    if let Err(e) = state.db.create_message(CreateMessage {
        conversation_id: conversation_id.to_string(),
        role: MessageRole::User,
        content: content.to_string(),
        tool_calls: None,
        tool_results: None,
    }) {
        tracing::error!("Failed to save user message: {}", e);
    }

    // Get response from agent
    let mut agent_guard = agent.lock().await;

    let result = if let Some(ref srvs) = servers {
        let srv_refs: Vec<&str> = srvs.iter().map(|s| s.as_str()).collect();
        agent_guard.chat_with_servers(content, &srv_refs).await
    } else {
        agent_guard.chat(content).await
    };

    drop(agent_guard);

    match result {
        Ok(response) => {
            // Save assistant message to database
            let msg = state.db.create_message(CreateMessage {
                conversation_id: conversation_id.to_string(),
                role: MessageRole::Assistant,
                content: response.clone(),
                tool_calls: None, // TODO: Extract from agent
                tool_results: None,
            });

            let message_id = msg.map(|m| m.id).unwrap_or_default();

            // Send complete message
            let server_msg = ServerMessage::Message {
                id: message_id,
                content: response,
                tool_calls: None,
                tool_results: None,
            };

            let _ = sender
                .lock()
                .await
                .send(Message::Text(serde_json::to_string(&server_msg).unwrap().into()))
                .await;
        }
        Err(e) => {
            let error_msg = ServerMessage::Error {
                message: e.to_string(),
            };
            let _ = sender
                .lock()
                .await
                .send(Message::Text(serde_json::to_string(&error_msg).unwrap().into()))
                .await;
        }
    }
}
