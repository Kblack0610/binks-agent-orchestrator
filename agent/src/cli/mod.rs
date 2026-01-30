//! CLI module
//!
//! This module provides:
//! - CLI argument definitions (args, mcps_args, workflow_args)
//! - Slash commands for interactive mode (commands)
//! - REPL functionality (repl)
//! - Mode management (modes)
//!
//! # Feature Availability
//!
//! - `modes`, `args` - Always available (no dependencies)
//! - `commands`, `repl` - Requires `mcp` feature (depends on Agent)
//! - `mcps_args` - Requires `mcp` feature
//! - `workflow_args` - Requires `orchestrator` feature

// =============================================================================
// Always available - CLI argument definitions
// =============================================================================
pub mod args;
pub mod modes;

pub use args::{Cli, Commands};
pub use modes::Mode;

// =============================================================================
// MCP feature - McpsCommands
// =============================================================================
#[cfg(feature = "mcp")]
pub mod mcps_args;
#[cfg(feature = "mcp")]
pub use mcps_args::McpsCommands;

// =============================================================================
// Orchestrator feature - WorkflowCommands, RunsCommands, SelfHealCommands
// =============================================================================
#[cfg(feature = "orchestrator")]
pub mod runs_args;
#[cfg(feature = "orchestrator")]
pub mod selfheal_args;
#[cfg(feature = "orchestrator")]
pub mod workflow_args;
#[cfg(feature = "orchestrator")]
pub use runs_args::RunsCommands;
#[cfg(feature = "orchestrator")]
pub use selfheal_args::SelfHealCommands;
#[cfg(feature = "orchestrator")]
pub use workflow_args::WorkflowCommands;

// =============================================================================
// Requires MCP feature (depends on Agent) - REPL and slash commands
// =============================================================================
#[cfg(feature = "mcp")]
pub mod commands;
#[cfg(feature = "mcp")]
pub mod repl;

#[cfg(feature = "mcp")]
pub use commands::{CommandContext, CommandRegistry, CommandResult, SlashCommand};
#[cfg(feature = "mcp")]
pub use repl::{Repl, ReplConfig};
