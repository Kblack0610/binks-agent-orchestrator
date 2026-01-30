//! Workflow MCP Server
//!
//! Provides workflow orchestration capabilities via MCP protocol.
//!
//! # Features
//!
//! - Execute multi-step agent workflows
//! - Handle checkpoints for human approval
//! - Track execution state and resume from checkpoints
//! - Load built-in and custom workflow definitions
//!
//! # Architecture
//!
//! - `types` - Core workflow type definitions
//! - `loader` - Load built-in and custom workflows from TOML
//! - `engine` - Workflow execution engine
//! - `handlers` - MCP tool handlers
//! - `params` - MCP parameter types
//! - `server` - MCP server implementation

pub mod engine;
pub mod handlers;
pub mod loader;
pub mod params;
pub mod server;
pub mod types;

// Re-export core types for convenience
pub use types::{
    StepResult, Workflow, WorkflowError, WorkflowResult, WorkflowStatus, WorkflowStep,
};
