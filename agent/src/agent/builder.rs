//! AgentBuilder - Ergonomic builder pattern for Agent construction
//!
//! # Example
//!
//! ```rust,ignore
//! use binks_agent::{AgentBuilder, Agent};
//! use sysinfo_mcp::SysinfoMcpServer;
//!
//! let agent = AgentBuilder::new()
//!     .with_ollama_url("http://localhost:11434")
//!     .with_model("qwen2.5:7b")
//!     .with_embedded_mcp(SysinfoMcpServer::new())
//!     .with_system_prompt("You are a helpful assistant.")
//!     .build()
//!     .await?;
//! ```

#[cfg(feature = "embedded")]
use std::sync::Arc;

use anyhow::Result;

use crate::config::AgentSectionConfig;
use crate::mcp::McpClientPool;

use super::{
    Agent, EventSender, ModelCapabilities, DEFAULT_LLM_TIMEOUT_SECS, DEFAULT_MAX_HISTORY_MESSAGES,
    DEFAULT_MAX_ITERATIONS, DEFAULT_TOOL_TIMEOUT_SECS,
};

#[cfg(feature = "embedded")]
use mcp_common::EmbeddableMcp;

/// Builder for constructing Agent instances with embedded MCPs
pub struct AgentBuilder {
    // LLM configuration
    ollama_url: String,
    model: String,

    // Agent settings
    system_prompt: Option<String>,
    max_iterations: usize,
    llm_timeout_secs: u64,
    tool_timeout_secs: u64,
    max_history_messages: usize,
    verbose: bool,
    event_sender: Option<EventSender>,
    capabilities: Option<ModelCapabilities>,

    // MCP configuration
    #[cfg(feature = "embedded")]
    embedded_mcps: Vec<Arc<dyn EmbeddableMcp>>,

    // Subprocess MCP pool (for mixed mode)
    mcp_pool: Option<McpClientPool>,
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentBuilder {
    /// Create a new AgentBuilder with default settings
    pub fn new() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            model: String::new(),
            system_prompt: None,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            llm_timeout_secs: DEFAULT_LLM_TIMEOUT_SECS,
            tool_timeout_secs: DEFAULT_TOOL_TIMEOUT_SECS,
            max_history_messages: DEFAULT_MAX_HISTORY_MESSAGES,
            verbose: false,
            event_sender: None,
            capabilities: None,
            #[cfg(feature = "embedded")]
            embedded_mcps: Vec::new(),
            mcp_pool: None,
        }
    }

    /// Set the Ollama server URL
    pub fn with_ollama_url(mut self, url: &str) -> Self {
        self.ollama_url = url.to_string();
        self
    }

    /// Set the model name
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = Some(prompt.to_string());
        self
    }

    /// Set maximum iterations for tool-calling loop
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Set LLM request timeout in seconds
    pub fn with_llm_timeout_secs(mut self, secs: u64) -> Self {
        self.llm_timeout_secs = secs;
        self
    }

    /// Set tool execution timeout in seconds
    pub fn with_tool_timeout_secs(mut self, secs: u64) -> Self {
        self.tool_timeout_secs = secs;
        self
    }

    /// Set maximum history messages to retain
    pub fn with_max_history_messages(mut self, max: usize) -> Self {
        self.max_history_messages = max;
        self
    }

    /// Enable verbose timing output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set event sender for real-time visibility
    pub fn with_event_sender(mut self, sender: EventSender) -> Self {
        self.event_sender = Some(sender);
        self
    }

    /// Set model capabilities
    pub fn with_capabilities(mut self, capabilities: ModelCapabilities) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    /// Apply settings from an AgentSectionConfig
    pub fn with_agent_config(mut self, config: &AgentSectionConfig) -> Self {
        if let Some(prompt) = &config.system_prompt {
            self.system_prompt = Some(prompt.clone());
        }
        self.max_iterations = config.max_iterations;
        self.llm_timeout_secs = config.llm_timeout_secs;
        self.tool_timeout_secs = config.tool_timeout_secs;
        self.max_history_messages = config.max_history_messages;
        self
    }

    /// Register an embedded MCP server (in-process, no subprocess spawning)
    ///
    /// This is the key method for embedding MCPs directly into the agent.
    /// Multiple embedded MCPs can be registered by chaining this method.
    #[cfg(feature = "embedded")]
    pub fn with_embedded_mcp<S: EmbeddableMcp + 'static>(mut self, mcp: S) -> Self {
        self.embedded_mcps.push(Arc::new(mcp));
        self
    }

    /// Use an existing MCP client pool (for subprocess MCPs)
    ///
    /// This allows mixing embedded MCPs with subprocess-based MCPs.
    pub fn with_mcp_pool(mut self, pool: McpClientPool) -> Self {
        self.mcp_pool = Some(pool);
        self
    }

    /// Build the Agent
    ///
    /// Returns an error if no model is specified.
    pub async fn build(self) -> Result<Agent> {
        // Validate required fields
        if self.model.is_empty() {
            anyhow::bail!("AgentBuilder: model is required - use .with_model(\"model_name\")");
        }

        // Create or use provided MCP pool
        #[allow(unused_mut)] // mut only needed with embedded feature
        let mut mcp_pool = self.mcp_pool.unwrap_or_else(McpClientPool::empty);

        // Register embedded MCPs
        #[cfg(feature = "embedded")]
        {
            for embedded in self.embedded_mcps {
                mcp_pool.register_embedded_arc(embedded);
            }
        }

        // Create the agent
        let mut agent = Agent::with_config(
            &self.ollama_url,
            &self.model,
            mcp_pool,
            self.max_iterations,
            self.llm_timeout_secs,
            self.tool_timeout_secs,
            self.max_history_messages,
        );

        // Apply optional settings
        if let Some(prompt) = &self.system_prompt {
            agent = agent.with_system_prompt(prompt);
        }

        if self.verbose {
            agent = agent.with_verbose(true);
        }

        if let Some(sender) = self.event_sender {
            agent = agent.with_event_sender(sender);
        }

        if let Some(capabilities) = self.capabilities {
            agent = agent.with_capabilities(capabilities);
        }

        Ok(agent)
    }

    /// Build the Agent synchronously (blocking)
    ///
    /// This is useful for FFI and contexts where async is not available.
    /// Creates a new tokio runtime for the build operation.
    pub fn build_blocking(self) -> Result<Agent> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(self.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = AgentBuilder::new();
        assert_eq!(builder.ollama_url, "http://localhost:11434");
        assert_eq!(builder.max_iterations, DEFAULT_MAX_ITERATIONS);
    }

    #[test]
    fn test_builder_fluent_api() {
        let builder = AgentBuilder::new()
            .with_ollama_url("http://custom:11434")
            .with_model("test-model")
            .with_system_prompt("Test prompt")
            .with_max_iterations(5)
            .with_verbose(true);

        assert_eq!(builder.ollama_url, "http://custom:11434");
        assert_eq!(builder.model, "test-model");
        assert_eq!(builder.system_prompt, Some("Test prompt".to_string()));
        assert_eq!(builder.max_iterations, 5);
        assert!(builder.verbose);
    }

    #[tokio::test]
    async fn test_builder_requires_model() {
        let result = AgentBuilder::new().build().await;
        match result {
            Ok(_) => panic!("Expected error when model not set"),
            Err(e) => assert!(e.to_string().contains("model is required")),
        }
    }

    #[tokio::test]
    async fn test_builder_creates_agent() {
        let agent = AgentBuilder::new()
            .with_model("test-model")
            .with_system_prompt("Test")
            .build()
            .await;

        assert!(agent.is_ok());
        let agent = agent.unwrap();
        assert_eq!(agent.model(), "test-model");
        assert_eq!(agent.system_prompt(), Some("Test"));
    }
}
