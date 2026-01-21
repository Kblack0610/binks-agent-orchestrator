//! CLI enhancements for interactive mode
//!
//! This module provides slash commands, REPL functionality, and mode management
//! for the interactive CLI.

pub mod commands;
pub mod modes;
pub mod repl;

pub use commands::{CommandContext, CommandRegistry, CommandResult, SlashCommand};
pub use modes::Mode;
pub use repl::{Repl, ReplConfig};
