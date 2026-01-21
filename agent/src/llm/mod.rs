//! LLM abstraction layer

mod ollama;

pub use ollama::{list_models, ModelInfo, OllamaClient};

use anyhow::Result;
use async_trait::async_trait;

/// Message in a conversation
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Trait for LLM backends
#[async_trait]
pub trait Llm: Send + Sync {
    /// Send a single message and get a response
    async fn chat(&self, message: &str) -> Result<String>;

    /// Chat with conversation history
    async fn chat_with_history(&self, history: &mut Vec<Message>, message: &str) -> Result<String>;

    /// Get the model name
    fn model(&self) -> &str;
}
