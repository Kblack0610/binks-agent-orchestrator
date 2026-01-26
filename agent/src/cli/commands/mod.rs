//! Slash command system for interactive CLI
//!
//! This module provides a trait-based system for slash commands like `/help`, `/models`, etc.
//! Commands are registered in a registry and dispatched based on user input.

mod clear;
mod help;
mod implement;
mod models;
mod normal;
mod plan;
mod tools;

pub use clear::ClearCommand;
pub use help::HelpCommand;
pub use implement::ImplementCommand;
pub use models::ModelsCommand;
pub use normal::NormalCommand;
pub use plan::PlanCommand;
pub use tools::ToolsCommand;

use super::modes::Mode;
use crate::agent::Agent;
use crate::output::OutputWriter;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

// ============================================================================
// Command Context
// ============================================================================

/// Context passed to commands during execution
pub struct CommandContext<'a> {
    /// The agent instance
    pub agent: &'a mut Agent,
    /// The output writer
    pub output: &'a dyn OutputWriter,
    /// Current mode
    pub mode: &'a Mode,
    /// Server filter (if any)
    pub server_filter: Option<&'a [String]>,
}

// ============================================================================
// Command Result
// ============================================================================

/// Result of executing a command
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Command executed successfully
    Ok,
    /// Command executed, display this message
    Message(String),
    /// Switch to a new mode
    SwitchMode(Mode),
    /// Clear the screen/history
    Clear,
    /// Exit the REPL
    Exit,
    /// Continue to chat (command was not handled)
    Continue,
}

// ============================================================================
// SlashCommand Trait
// ============================================================================

/// Trait for slash commands
///
/// Implement this trait to add new slash commands to the CLI.
#[async_trait]
pub trait SlashCommand: Send + Sync {
    /// Command name (without the leading slash)
    fn name(&self) -> &'static str;

    /// Short description for help text
    fn description(&self) -> &'static str;

    /// Aliases for this command (e.g., ["q"] for quit)
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    /// Whether this command is available in the given mode
    fn available_in(&self, _mode: &Mode) -> bool {
        true
    }

    /// Execute the command
    async fn execute(&self, args: &str, ctx: &mut CommandContext<'_>) -> Result<CommandResult>;
}

// ============================================================================
// Command Registry
// ============================================================================

/// Registry of slash commands
pub struct CommandRegistry {
    commands: Vec<Arc<dyn SlashCommand>>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    /// Create a new registry with all built-in commands
    pub fn new() -> Self {
        let commands: Vec<Arc<dyn SlashCommand>> = vec![
            Arc::new(HelpCommand),
            Arc::new(ModelsCommand),
            Arc::new(ToolsCommand),
            Arc::new(ClearCommand),
            Arc::new(PlanCommand),
            Arc::new(ImplementCommand),
            Arc::new(NormalCommand),
        ];

        Self { commands }
    }

    /// Register a custom command
    pub fn register(&mut self, command: Arc<dyn SlashCommand>) {
        self.commands.push(command);
    }

    /// Find a command by name or alias
    pub fn find(&self, name: &str) -> Option<&Arc<dyn SlashCommand>> {
        for cmd in &self.commands {
            if cmd.name() == name {
                return Some(cmd);
            }
            for alias in cmd.aliases() {
                if *alias == name {
                    return Some(cmd);
                }
            }
        }
        None
    }

    /// Check if input is a command (starts with /)
    pub fn is_command(input: &str) -> bool {
        input.starts_with('/')
    }

    /// Parse command input into (command_name, args)
    pub fn parse_command(input: &str) -> Option<(&str, &str)> {
        if !Self::is_command(input) {
            return None;
        }

        let input = input.trim_start_matches('/');
        let mut parts = input.splitn(2, char::is_whitespace);
        let name = parts.next()?;
        let args = parts.next().unwrap_or("").trim();

        Some((name, args))
    }

    /// Get all commands available in the given mode
    pub fn available_commands(&self, mode: &Mode) -> Vec<&Arc<dyn SlashCommand>> {
        self.commands
            .iter()
            .filter(|cmd| cmd.available_in(mode))
            .collect()
    }

    /// Get all registered commands
    pub fn all_commands(&self) -> &[Arc<dyn SlashCommand>] {
        &self.commands
    }

    /// Try to execute a command
    ///
    /// Returns `Some(result)` if input was a command, `None` if not a command.
    pub async fn try_execute(
        &self,
        input: &str,
        ctx: &mut CommandContext<'_>,
    ) -> Option<Result<CommandResult>> {
        let (name, args) = Self::parse_command(input)?;

        let cmd = match self.find(name) {
            Some(cmd) => cmd,
            None => {
                return Some(Ok(CommandResult::Message(format!(
                    "Unknown command: /{}. Type /help for available commands.",
                    name
                ))));
            }
        };

        if !cmd.available_in(ctx.mode) {
            return Some(Ok(CommandResult::Message(format!(
                "Command /{} is not available in {} mode",
                name,
                ctx.mode.name()
            ))));
        }

        Some(cmd.execute(args, ctx).await)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_command() {
        assert!(CommandRegistry::is_command("/help"));
        assert!(CommandRegistry::is_command("/models"));
        assert!(!CommandRegistry::is_command("hello"));
        assert!(!CommandRegistry::is_command(""));
    }

    #[test]
    fn test_parse_command() {
        assert_eq!(CommandRegistry::parse_command("/help"), Some(("help", "")));
        assert_eq!(
            CommandRegistry::parse_command("/models list"),
            Some(("models", "list"))
        );
        assert_eq!(
            CommandRegistry::parse_command("/plan some context here"),
            Some(("plan", "some context here"))
        );
        assert_eq!(CommandRegistry::parse_command("not a command"), None);
    }

    #[test]
    fn test_registry_creation() {
        let registry = CommandRegistry::new();
        assert!(!registry.all_commands().is_empty());
    }

    #[test]
    fn test_find_command() {
        let registry = CommandRegistry::new();

        assert!(registry.find("help").is_some());
        assert!(registry.find("models").is_some());
        assert!(registry.find("nonexistent").is_none());
    }
}
