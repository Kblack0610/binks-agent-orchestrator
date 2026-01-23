//! CLI enhancements for interactive mode
//!
//! This module provides slash commands, REPL functionality, and mode management
//! for the interactive CLI.
//!
//! # Feature Availability
//!
//! - `modes` - Always available (no dependencies)
//! - `commands`, `repl` - Requires `mcp` feature (depends on Agent)

// =============================================================================
// Always available
// =============================================================================
pub mod modes;
pub use modes::Mode;

// =============================================================================
// Requires MCP feature (depends on Agent)
// =============================================================================
#[cfg(feature = "mcp")]
pub mod commands;
#[cfg(feature = "mcp")]
pub mod repl;

#[cfg(feature = "mcp")]
pub use commands::{CommandContext, CommandRegistry, CommandResult, SlashCommand};
#[cfg(feature = "mcp")]
pub use repl::{Repl, ReplConfig};
