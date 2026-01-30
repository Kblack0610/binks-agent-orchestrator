//! Multi-agent workflow orchestration module
//!
//! This module provides:
//! - Agent configuration with per-agent model selection
//! - Agent prompts for different workflow roles
//!
//! # Note
//!
//! Workflow execution functionality has been extracted to workflow-mcp.
//! For workflow execution, use the workflow-mcp MCP server via WorkflowClient.

pub mod agent_config;
pub mod prompts;

// Legacy workflow engine modules - retained for web UI compatibility
// These are only compiled when orchestrator feature is enabled
#[cfg(feature = "orchestrator")]
pub mod checkpoint;
#[cfg(feature = "orchestrator")]
pub mod engine;
#[cfg(feature = "orchestrator")]
pub mod workflow;

pub use agent_config::{AgentConfig, AgentRegistry};

#[cfg(feature = "orchestrator")]
pub use checkpoint::{Checkpoint, CheckpointResult};
#[cfg(feature = "orchestrator")]
pub use engine::{EngineConfig, WorkflowEngine};
#[cfg(feature = "orchestrator")]
pub use workflow::{StepResult, Workflow, WorkflowResult, WorkflowStep};
