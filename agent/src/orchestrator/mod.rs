//! Multi-agent workflow orchestration module
//!
//! This module provides:
//! - Agent configuration with per-agent model selection
//! - Workflow primitives (sequential, checkpoint, parallel)
//! - Workflow engine for executing multi-agent flows
//! - Built-in workflows and custom TOML workflow support
//!
//! # Example
//!
//! ```rust,ignore
//! use agent::orchestrator::{WorkflowEngine, EngineConfig};
//!
//! let config = EngineConfig {
//!     ollama_url: "http://localhost:11434".to_string(),
//!     default_model: "qwen2.5-coder:14b".to_string(),
//!     ..Default::default()
//! };
//! let engine = WorkflowEngine::new(config)?;
//!
//! let result = engine
//!     .run("implement-feature", "Add dark mode toggle")
//!     .await?;
//! ```

pub mod agent_config;
pub mod checkpoint;
pub mod engine;
pub mod prompts;
pub mod workflow;

pub use agent_config::{AgentConfig, AgentRegistry};
pub use checkpoint::{Checkpoint, CheckpointResult};
pub use engine::{EngineConfig, WorkflowEngine};
pub use workflow::{StepResult, Workflow, WorkflowResult, WorkflowStep};
