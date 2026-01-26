//! Ollama LLM implementation

use anyhow::Result;
use async_trait::async_trait;
use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage},
    Ollama,
};
use serde::{Deserialize, Serialize};

use super::{Llm, Message, Role};

/// Information about an available model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub modified_at: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<ModelInfo>,
}

/// List available models from Ollama
pub async fn list_models(ollama_url: &str) -> Result<Vec<ModelInfo>> {
    let url = url::Url::parse(ollama_url)
        .unwrap_or_else(|_| url::Url::parse("http://localhost:11434").unwrap());

    let client = reqwest::Client::new();
    let api_url = format!("{}api/tags", url);

    let response: OllamaTagsResponse = client.get(&api_url).send().await?.json().await?;

    Ok(response.models)
}

/// Ollama client wrapper
pub struct OllamaClient {
    client: Ollama,
    model: String,
}

impl OllamaClient {
    /// Create a new Ollama client
    pub fn new(url: &str, model: &str) -> Self {
        // Parse URL to extract host and port
        let url = url::Url::parse(url)
            .unwrap_or_else(|_| url::Url::parse("http://localhost:11434").unwrap());

        let host = url.host_str().unwrap_or("localhost").to_string();
        let port = url.port().unwrap_or(11434);

        Self {
            client: Ollama::new(format!("http://{}", host), port),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl Llm for OllamaClient {
    async fn chat(&self, message: &str) -> Result<String> {
        let request = ChatMessageRequest::new(
            self.model.clone(),
            vec![ChatMessage::user(message.to_string())],
        );

        let response = self.client.send_chat_messages(request).await?;

        Ok(response.message.content)
    }

    async fn chat_with_history(&self, history: &mut Vec<Message>, message: &str) -> Result<String> {
        // Add user message to history
        history.push(Message {
            role: Role::User,
            content: message.to_string(),
        });

        // Convert history to Ollama format
        let messages: Vec<ChatMessage> = history
            .iter()
            .map(|m| match m.role {
                Role::System => ChatMessage::system(m.content.clone()),
                Role::User => ChatMessage::user(m.content.clone()),
                Role::Assistant => ChatMessage::assistant(m.content.clone()),
            })
            .collect();

        let request = ChatMessageRequest::new(self.model.clone(), messages);

        let response = self.client.send_chat_messages(request).await?;

        // Add assistant response to history
        history.push(Message {
            role: Role::Assistant,
            content: response.message.content.clone(),
        });

        Ok(response.message.content)
    }

    fn model(&self) -> &str {
        &self.model
    }
}
