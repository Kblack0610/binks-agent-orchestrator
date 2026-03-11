//! LiteLLM gateway client and model catalog helpers.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{Llm, Message, Role};

/// Capabilities surfaced by the gateway model catalog.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilitiesInfo {
    #[serde(default)]
    pub tools: bool,
    #[serde(default)]
    pub vision: bool,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub json_mode: bool,
    #[serde(default)]
    pub code: bool,
}

/// Normalized model entry returned to web and CLI clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub provider: String,
    pub provider_model_id: String,
    pub group: String,
    #[serde(default)]
    pub capabilities: ModelCapabilitiesInfo,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Deserialize)]
struct GatewayModelsResponse {
    data: Vec<GatewayModelEntry>,
}

#[derive(Debug, Deserialize)]
struct GatewayModelEntry {
    id: String,
    #[serde(default)]
    owned_by: String,
    #[serde(default)]
    litellm_provider: Option<String>,
    #[serde(default)]
    model_name: Option<String>,
    #[serde(default)]
    metadata: serde_json::Value,
    #[serde(default)]
    model_info: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct GatewayChatRequest {
    model: String,
    messages: Vec<GatewayChatMessage>,
}

#[derive(Debug, Serialize)]
struct GatewayChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GatewayChatResponse {
    choices: Vec<GatewayChoice>,
}

#[derive(Debug, Deserialize)]
struct GatewayChoice {
    message: GatewayResponseMessage,
}

#[derive(Debug, Deserialize)]
struct GatewayResponseMessage {
    #[serde(default)]
    content: Option<String>,
}

fn trim_trailing_slash(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

fn infer_reasoning(model: &str) -> bool {
    let model = model.to_lowercase();
    model.contains("deepseek-r1")
        || model.contains("qwq")
        || model.contains("-r1")
        || model.contains("reasoning")
}

fn infer_code(model: &str) -> bool {
    let model = model.to_lowercase();
    model.contains("code") || model.contains("coder")
}

fn infer_vision(model: &str) -> bool {
    let model = model.to_lowercase();
    model.contains("vision") || model.contains("llava")
}

fn extract_bool(value: &serde_json::Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        value
            .get(key)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    })
}

fn extract_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string)
    })
}

fn extract_string_list(value: &serde_json::Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .find_map(|key| value.get(key))
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn infer_provider(model_id: &str, entry: &GatewayModelEntry) -> String {
    entry
        .litellm_provider
        .clone()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            if !entry.owned_by.is_empty() {
                Some(entry.owned_by.clone())
            } else {
                None
            }
        })
        .or_else(|| model_id.split('/').next().map(ToString::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn provider_label(provider: &str) -> Option<&'static str> {
    match provider {
        "ollama" | "mlx" | "lm_studio" | "openai_compatible" => Some("local"),
        _ => None,
    }
}

fn normalize_model(entry: GatewayModelEntry, default_model: Option<&str>) -> ModelInfo {
    let provider = infer_provider(&entry.id, &entry);
    let provider_model_id = entry
        .model_name
        .clone()
        .or_else(|| entry.id.split_once('/').map(|(_, model)| model.to_string()))
        .unwrap_or_else(|| entry.id.clone());

    let display_name = extract_string(&entry.metadata, &["display_name", "name", "label"])
        .unwrap_or_else(|| provider_model_id.clone());

    let mut labels = extract_string_list(&entry.metadata, &["labels", "tags"]);
    if let Some(locality) = provider_label(provider.as_str()) {
        if !labels.iter().any(|label| label == locality) {
            labels.push(locality.to_string());
        }
    }

    let capabilities = ModelCapabilitiesInfo {
        tools: extract_bool(
            &entry.model_info,
            &[
                "supports_function_calling",
                "supports_tool_choice",
                "supports_parallel_function_calling",
            ],
        ) || extract_bool(&entry.metadata, &["tools", "supports_tools"]),
        vision: extract_bool(&entry.model_info, &["supports_vision"])
            || extract_bool(&entry.metadata, &["vision"])
            || infer_vision(&entry.id),
        reasoning: extract_bool(&entry.metadata, &["reasoning"]) || infer_reasoning(&entry.id),
        json_mode: extract_bool(
            &entry.model_info,
            &["supports_response_schema", "supports_json_schema"],
        ) || extract_bool(&entry.metadata, &["json_mode"]),
        code: extract_bool(&entry.metadata, &["code"]) || infer_code(&entry.id),
    };

    ModelInfo {
        id: entry.id.clone(),
        display_name,
        provider: provider.clone(),
        provider_model_id,
        group: provider,
        capabilities,
        labels,
        is_default: default_model.is_some_and(|model| model == entry.id),
    }
}

/// List available models from the LiteLLM gateway.
pub async fn list_models(gateway_url: &str, default_model: Option<&str>) -> Result<Vec<ModelInfo>> {
    let client = reqwest::Client::new();
    let response: GatewayModelsResponse = client
        .get(format!("{}/v1/models", trim_trailing_slash(gateway_url)))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(response
        .data
        .into_iter()
        .map(|entry| normalize_model(entry, default_model))
        .collect())
}

/// Gateway model inspection is not implemented beyond catalog discovery.
pub async fn show_model(
    _gateway_url: &str,
    _model: &str,
) -> Result<crate::agent::capabilities::OllamaShowResponse> {
    anyhow::bail!("Gateway model inspection is not supported; use catalog metadata or overrides");
}

/// LiteLLM gateway client wrapper.
pub struct OllamaClient {
    client: reqwest::Client,
    gateway_url: String,
    model: String,
}

impl OllamaClient {
    /// Create a new gateway client.
    pub fn new(url: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            gateway_url: trim_trailing_slash(url),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl Llm for OllamaClient {
    async fn chat(&self, message: &str) -> Result<String> {
        let request = GatewayChatRequest {
            model: self.model.clone(),
            messages: vec![GatewayChatMessage {
                role: "user".to_string(),
                content: message.to_string(),
            }],
        };

        let response: GatewayChatResponse = self
            .client
            .post(format!("{}/v1/chat/completions", self.gateway_url))
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("Failed to parse gateway response")?;

        Ok(response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_default())
    }

    async fn chat_with_history(&self, history: &mut Vec<Message>, message: &str) -> Result<String> {
        history.push(Message {
            role: Role::User,
            content: message.to_string(),
        });

        let messages: Vec<GatewayChatMessage> = history
            .iter()
            .map(|message| GatewayChatMessage {
                role: match message.role {
                    Role::System => "system".to_string(),
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                },
                content: message.content.clone(),
            })
            .collect();

        let response: GatewayChatResponse = self
            .client
            .post(format!("{}/v1/chat/completions", self.gateway_url))
            .json(&GatewayChatRequest {
                model: self.model.clone(),
                messages,
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("Failed to parse gateway response")?;

        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_default();

        history.push(Message {
            role: Role::Assistant,
            content: content.clone(),
        });

        Ok(content)
    }

    fn model(&self) -> &str {
        &self.model
    }
}
