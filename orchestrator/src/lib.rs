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
//! use orchestrator::{WorkflowEngine, EngineConfig, AgentRegistry};
//!
//! let registry = AgentRegistry::with_defaults("qwen2.5-coder:14b");
//! let config = EngineConfig::default();
//! let engine = WorkflowEngine::new(registry, config);
//!
//! let result = engine
//!     .run("implement-feature", "Add dark mode toggle")
//!     .await?;
//! ```

// Re-export everything from agent::orchestrator
// This crate exists primarily for the CLI binary and backward compatibility
pub use agent::orchestrator::*;

// Re-export commonly used types from the agent crate
pub use agent::agent::Agent;
pub use agent::config::{AgentFileConfig, McpConfig};
pub use agent::mcp::McpClientPool;
