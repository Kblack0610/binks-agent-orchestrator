//! Multi-agent workflow orchestration layer for binks-agent
//!
//! This crate provides:
//! - Agent configuration with per-agent model selection
//! - Workflow primitives (sequential, checkpoint, parallel)
//! - Workflow engine for executing multi-agent flows
//! - Built-in workflows and custom TOML workflow support
//!
//! # Example
//!
//! ```rust,ignore
//! use orchestrator::{WorkflowEngine, Workflow, AgentRegistry};
//!
//! let registry = AgentRegistry::default();
//! let engine = WorkflowEngine::new(registry);
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
pub use engine::WorkflowEngine;
pub use workflow::{Workflow, WorkflowStep, WorkflowResult, StepResult};

/// Re-export commonly used types from the agent crate
pub use agent::agent::Agent;
pub use agent::config::{AgentFileConfig, McpConfig};
pub use agent::mcp::McpClientPool;
