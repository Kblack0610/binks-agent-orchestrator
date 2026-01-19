//! Agent configuration and registry
//!
//! Defines specialized agents with per-agent model selection,
//! system prompts, and tool filtering.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::prompts;

/// Configuration for a specialized agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique identifier for this agent
    pub name: String,

    /// Display name for UI/logging
    #[serde(default)]
    pub display_name: String,

    /// LLM model to use (e.g., "qwen3:14b", "llama3.1:70b")
    pub model: String,

    /// System prompt defining agent behavior
    pub system_prompt: String,

    /// MCP servers this agent can use (empty = all servers)
    #[serde(default)]
    pub tools: Vec<String>,

    /// Temperature for LLM sampling (0.0 = deterministic, 1.0 = creative)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Maximum tokens to generate
    #[serde(default)]
    pub max_tokens: Option<u32>,

    /// Agents this agent can hand off to
    #[serde(default)]
    pub can_handoff_to: Vec<String>,
}

fn default_temperature() -> f32 {
    0.7
}

impl AgentConfig {
    /// Create a new agent configuration
    pub fn new(name: impl Into<String>, model: impl Into<String>, system_prompt: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            display_name: name.clone(),
            name,
            model: model.into(),
            system_prompt: system_prompt.into(),
            tools: Vec::new(),
            temperature: default_temperature(),
            max_tokens: None,
            can_handoff_to: Vec::new(),
        }
    }

    /// Set the display name
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    /// Set allowed MCP servers (tools)
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set agents this agent can hand off to
    pub fn with_handoffs(mut self, agents: Vec<String>) -> Self {
        self.can_handoff_to = agents;
        self
    }
}

/// Registry of available agents
#[derive(Debug, Clone, Default)]
pub struct AgentRegistry {
    agents: HashMap<String, AgentConfig>,
}

impl AgentRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Create a registry with built-in agents
    pub fn with_defaults(default_model: impl Into<String>) -> Self {
        let model = default_model.into();
        let mut registry = Self::new();

        // Planner agent - analyzes tasks and creates implementation plans
        registry.register(
            AgentConfig::new("planner", &model, prompts::PLANNER_PROMPT)
                .with_display_name("Planner")
                .with_tools(vec![
                    "filesystem".to_string(),
                    "serena".to_string(),
                ])
                .with_temperature(0.3)
                .with_handoffs(vec!["implementer".to_string(), "researcher".to_string()]),
        );

        // Implementer agent - makes code changes based on plans
        registry.register(
            AgentConfig::new("implementer", &model, prompts::IMPLEMENTER_PROMPT)
                .with_display_name("Implementer")
                .with_tools(vec![
                    "filesystem".to_string(),
                    "serena".to_string(),
                    "github-gh".to_string(),
                ])
                .with_temperature(0.2)
                .with_handoffs(vec!["reviewer".to_string(), "planner".to_string()]),
        );

        // Reviewer agent - reviews changes and provides feedback
        registry.register(
            AgentConfig::new("reviewer", &model, prompts::REVIEWER_PROMPT)
                .with_display_name("Reviewer")
                .with_tools(vec![
                    "filesystem".to_string(),
                    "serena".to_string(),
                    "github-gh".to_string(),
                ])
                .with_temperature(0.4)
                .with_handoffs(vec!["implementer".to_string(), "planner".to_string()]),
        );

        // Investigator agent - debugs issues and traces problems
        registry.register(
            AgentConfig::new("investigator", &model, prompts::INVESTIGATOR_PROMPT)
                .with_display_name("Investigator")
                .with_tools(vec![
                    "filesystem".to_string(),
                    "serena".to_string(),
                    "sysinfo".to_string(),
                ])
                .with_temperature(0.3)
                .with_handoffs(vec!["implementer".to_string(), "planner".to_string()]),
        );

        // Tester agent - runs tests and validates changes
        registry.register(
            AgentConfig::new("tester", &model, prompts::TESTER_PROMPT)
                .with_display_name("Tester")
                .with_tools(vec![
                    "filesystem".to_string(),
                    "serena".to_string(),
                ])
                .with_temperature(0.1)
                .with_handoffs(vec!["implementer".to_string(), "reviewer".to_string()]),
        );

        registry
    }

    /// Register a new agent
    pub fn register(&mut self, config: AgentConfig) {
        self.agents.insert(config.name.clone(), config);
    }

    /// Get an agent by name
    pub fn get(&self, name: &str) -> Option<&AgentConfig> {
        self.agents.get(name)
    }

    /// Get a mutable reference to an agent
    pub fn get_mut(&mut self, name: &str) -> Option<&mut AgentConfig> {
        self.agents.get_mut(name)
    }

    /// Check if an agent exists
    pub fn contains(&self, name: &str) -> bool {
        self.agents.contains_key(name)
    }

    /// List all agent names
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.agents.keys().map(|s| s.as_str())
    }

    /// Iterate over all agents
    pub fn iter(&self) -> impl Iterator<Item = (&str, &AgentConfig)> {
        self.agents.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Update model for a specific agent
    pub fn set_model(&mut self, agent_name: &str, model: impl Into<String>) -> bool {
        if let Some(agent) = self.agents.get_mut(agent_name) {
            agent.model = model.into();
            true
        } else {
            false
        }
    }

    /// Update model for all agents
    pub fn set_all_models(&mut self, model: impl Into<String>) {
        let model = model.into();
        for agent in self.agents.values_mut() {
            agent.model = model.clone();
        }
    }
}

/// Configuration file format for agent registry
#[derive(Debug, Deserialize)]
pub struct AgentRegistryConfig {
    /// Default model for agents that don't specify one
    #[serde(default = "default_model")]
    pub default_model: String,

    /// Agent configurations
    #[serde(default)]
    pub agents: Vec<AgentConfig>,
}

fn default_model() -> String {
    "qwen3:14b".to_string()
}

impl AgentRegistryConfig {
    /// Load from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Convert to AgentRegistry
    pub fn into_registry(self) -> AgentRegistry {
        let mut registry = AgentRegistry::with_defaults(&self.default_model);

        // Override with custom agents
        for agent in self.agents {
            registry.register(agent);
        }

        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_builder() {
        let config = AgentConfig::new("test", "gpt-4", "You are a test agent")
            .with_display_name("Test Agent")
            .with_tools(vec!["tool1".to_string()])
            .with_temperature(0.5)
            .with_handoffs(vec!["other".to_string()]);

        assert_eq!(config.name, "test");
        assert_eq!(config.display_name, "Test Agent");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.tools, vec!["tool1"]);
        assert_eq!(config.can_handoff_to, vec!["other"]);
    }

    #[test]
    fn test_registry_defaults() {
        let registry = AgentRegistry::with_defaults("test-model");

        assert!(registry.contains("planner"));
        assert!(registry.contains("implementer"));
        assert!(registry.contains("reviewer"));
        assert!(registry.contains("investigator"));
        assert!(registry.contains("tester"));

        let planner = registry.get("planner").unwrap();
        assert_eq!(planner.model, "test-model");
    }

    #[test]
    fn test_registry_set_models() {
        let mut registry = AgentRegistry::with_defaults("old-model");
        registry.set_all_models("new-model");

        for (_, agent) in registry.iter() {
            assert_eq!(agent.model, "new-model");
        }
    }
}
