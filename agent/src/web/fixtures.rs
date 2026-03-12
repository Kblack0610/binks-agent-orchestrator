//! Deterministic web fixture scenarios for browser E2E testing.

use std::{env, sync::Arc, time::Duration};

use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;

use crate::{
    db::messages::{ToolCall, ToolResult},
    llm::{ModelCapabilitiesInfo, ModelInfo},
};

#[derive(Debug, Clone)]
pub struct FixtureToolInfo {
    pub name: String,
    pub server: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureScenario {
    Healthy,
    ModelsUnavailable,
    WsError,
    ToolFlow,
}

impl FixtureScenario {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "healthy" => Some(Self::Healthy),
            "models-unavailable" | "models_unavailable" => Some(Self::ModelsUnavailable),
            "ws-error" | "ws_error" => Some(Self::WsError),
            "tool-flow" | "tool_flow" => Some(Self::ToolFlow),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebFixture {
    pub scenario: FixtureScenario,
    pub gateway_url: String,
    pub gateway_type: String,
    pub default_model: String,
    pub models: Vec<ModelInfo>,
    pub tools: Vec<FixtureToolInfo>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum FixtureServerMessage {
    #[serde(rename = "connected")]
    Connected { conversation_id: String },
    #[serde(rename = "token")]
    Token { content: String },
    #[serde(rename = "tool_start")]
    ToolStart {
        name: String,
        arguments: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        name: String,
        result: String,
        is_error: bool,
    },
    #[serde(rename = "message")]
    Message {
        id: String,
        content: String,
        tool_calls: Option<Vec<ToolCall>>,
        tool_results: Option<Vec<ToolResult>>,
    },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "pong")]
    Pong,
}

impl WebFixture {
    pub fn from_env() -> Option<Arc<Self>> {
        let scenario = env::var("BINKS_WEB_FIXTURE_SCENARIO")
            .ok()
            .and_then(|value| FixtureScenario::parse(&value))?;

        let default_model =
            env::var("BINKS_WEB_FIXTURE_MODEL").unwrap_or_else(|_| "code".to_string());

        Some(Arc::new(Self {
            scenario,
            gateway_url: env::var("BINKS_WEB_FIXTURE_GATEWAY_URL")
                .unwrap_or_else(|_| "http://fixture-gateway.local".to_string()),
            gateway_type: "fixture".to_string(),
            default_model: default_model.clone(),
            models: fixture_models(&default_model),
            tools: fixture_tools(),
        }))
    }

    pub fn list_models(&self) -> Result<Vec<ModelInfo>> {
        if self.scenario == FixtureScenario::ModelsUnavailable {
            anyhow::bail!("Fixture scenario configured to fail model listing");
        }

        Ok(self.models.clone())
    }

    pub async fn handle_socket(&self, socket: WebSocket, conversation_id: String) {
        let (mut sender, mut receiver) = socket.split();

        if send_json(
            &mut sender,
            &FixtureServerMessage::Connected {
                conversation_id: conversation_id.clone(),
            },
        )
        .await
        .is_err()
        {
            return;
        }

        while let Some(Ok(message)) = receiver.next().await {
            match message {
                Message::Text(text) => {
                    let value: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(value) => value,
                        Err(_) => continue,
                    };

                    match value.get("type").and_then(serde_json::Value::as_str) {
                        Some("ping") => {
                            let _ = send_json(&mut sender, &FixtureServerMessage::Pong).await;
                        }
                        Some("message") => {
                            let content = value
                                .get("content")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default();
                            let selected_model = value
                                .get("model")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or(self.default_model.as_str());

                            if self.scenario == FixtureScenario::WsError {
                                let _ = send_json(
                                    &mut sender,
                                    &FixtureServerMessage::Error {
                                        message: "Fixture websocket error".to_string(),
                                    },
                                )
                                .await;
                                let _ = sender.close().await;
                                return;
                            }

                            if self.scenario == FixtureScenario::ToolFlow {
                                let _ = send_json(
                                    &mut sender,
                                    &FixtureServerMessage::ToolStart {
                                        name: "sysinfo".to_string(),
                                        arguments: serde_json::json!({ "scope": "system" }),
                                    },
                                )
                                .await;
                                tokio::time::sleep(Duration::from_millis(25)).await;
                                let _ = send_json(
                                    &mut sender,
                                    &FixtureServerMessage::ToolResult {
                                        name: "sysinfo".to_string(),
                                        result: "cpu: 23%".to_string(),
                                        is_error: false,
                                    },
                                )
                                .await;
                            } else {
                                let _ = send_json(
                                    &mut sender,
                                    &FixtureServerMessage::Token {
                                        content: "fixture ".to_string(),
                                    },
                                )
                                .await;
                                tokio::time::sleep(Duration::from_millis(10)).await;
                                let _ = send_json(
                                    &mut sender,
                                    &FixtureServerMessage::Token {
                                        content: "response".to_string(),
                                    },
                                )
                                .await;
                            }

                            let reply = if self.scenario == FixtureScenario::ToolFlow {
                                format!("tool flow complete using {}", selected_model)
                            } else {
                                format!(
                                    "fixture response to '{}' using {}",
                                    content, selected_model
                                )
                            };

                            let _ = send_json(
                                &mut sender,
                                &FixtureServerMessage::Message {
                                    id: format!("fixture-{}", conversation_id),
                                    content: reply,
                                    tool_calls: None,
                                    tool_results: None,
                                },
                            )
                            .await;
                        }
                        _ => {}
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    }
}

async fn send_json(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    payload: &FixtureServerMessage,
) -> Result<()> {
    sender
        .send(Message::Text(serde_json::to_string(payload)?.into()))
        .await?;
    Ok(())
}

fn fixture_models(default_model: &str) -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "code".to_string(),
            display_name: "Code".to_string(),
            provider: "alias".to_string(),
            provider_model_id: "code".to_string(),
            group: "aliases".to_string(),
            capabilities: ModelCapabilitiesInfo {
                tools: true,
                vision: false,
                reasoning: false,
                json_mode: true,
                code: true,
            },
            labels: vec!["fast".to_string(), "alias".to_string()],
            is_default: default_model == "code",
        },
        ModelInfo {
            id: "mlx/qwen3-coder".to_string(),
            display_name: "Qwen3 Coder".to_string(),
            provider: "mlx".to_string(),
            provider_model_id: "qwen3-coder".to_string(),
            group: "mlx".to_string(),
            capabilities: ModelCapabilitiesInfo {
                tools: true,
                vision: false,
                reasoning: false,
                json_mode: true,
                code: true,
            },
            labels: vec!["local".to_string(), "code".to_string()],
            is_default: default_model == "mlx/qwen3-coder",
        },
        ModelInfo {
            id: "mlx/deepseek-r1".to_string(),
            display_name: "DeepSeek R1".to_string(),
            provider: "mlx".to_string(),
            provider_model_id: "deepseek-r1".to_string(),
            group: "mlx".to_string(),
            capabilities: ModelCapabilitiesInfo {
                tools: false,
                vision: false,
                reasoning: true,
                json_mode: false,
                code: false,
            },
            labels: vec!["local".to_string(), "reasoning".to_string()],
            is_default: default_model == "mlx/deepseek-r1",
        },
        ModelInfo {
            id: "openai/gpt-4o-mini".to_string(),
            display_name: "GPT-4o Mini".to_string(),
            provider: "openai".to_string(),
            provider_model_id: "gpt-4o-mini".to_string(),
            group: "openai".to_string(),
            capabilities: ModelCapabilitiesInfo {
                tools: true,
                vision: true,
                reasoning: false,
                json_mode: true,
                code: false,
            },
            labels: vec!["hosted".to_string()],
            is_default: default_model == "openai/gpt-4o-mini",
        },
    ]
}

fn fixture_tools() -> Vec<FixtureToolInfo> {
    vec![
        FixtureToolInfo {
            name: "sysinfo".to_string(),
            server: "sysinfo".to_string(),
            description: Some("System information".to_string()),
        },
        FixtureToolInfo {
            name: "github-search".to_string(),
            server: "github".to_string(),
            description: Some("Search GitHub issues".to_string()),
        },
    ]
}
