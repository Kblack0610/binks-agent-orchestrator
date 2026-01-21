//! REPL (Read-Eval-Print Loop) for interactive CLI
//!
//! This module provides the main interactive loop with slash command support,
//! mode management, and output handling.

use std::io::{self, BufRead, Write};

use anyhow::Result;

use super::commands::{CommandContext, CommandRegistry, CommandResult};
use super::modes::Mode;
use crate::agent::Agent;
use crate::output::{OutputEvent, OutputWriter};

/// REPL configuration
#[derive(Debug, Clone)]
pub struct ReplConfig {
    /// Server filter (optional)
    pub server_filter: Option<Vec<String>>,
    /// Initial mode
    pub initial_mode: Mode,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            server_filter: None,
            initial_mode: Mode::Normal,
        }
    }
}

/// Interactive REPL
pub struct Repl<'a> {
    agent: &'a mut Agent,
    output: &'a dyn OutputWriter,
    command_registry: CommandRegistry,
    mode: Mode,
    server_filter: Option<Vec<String>>,
}

impl<'a> Repl<'a> {
    /// Create a new REPL
    pub fn new(agent: &'a mut Agent, output: &'a dyn OutputWriter) -> Self {
        Self {
            agent,
            output,
            command_registry: CommandRegistry::new(),
            mode: Mode::Normal,
            server_filter: None,
        }
    }

    /// Configure the REPL
    pub fn with_config(mut self, config: ReplConfig) -> Self {
        self.server_filter = config.server_filter;
        self.mode = config.initial_mode;
        self
    }

    /// Set server filter
    pub fn with_server_filter(mut self, servers: Vec<String>) -> Self {
        self.server_filter = Some(servers);
        self
    }

    /// Run the REPL loop
    pub async fn run(&mut self) -> Result<()> {
        // Print welcome message
        self.output.write(OutputEvent::System(
            "Interactive agent mode. Type /help for commands, 'quit' to exit.".to_string(),
        ));
        self.output.write(OutputEvent::NewLine);

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            // Print prompt
            let prompt = format!("{}> ", self.mode.prompt_prefix());
            print!("{}", prompt);
            stdout.flush()?;

            // Read input
            let mut input = String::new();
            stdin.lock().read_line(&mut input)?;
            let input = input.trim();

            // Handle empty input
            if input.is_empty() {
                continue;
            }

            // Handle quit
            if input == "quit" || input == "exit" {
                break;
            }

            // Handle commands
            if CommandRegistry::is_command(input) {
                let result = self.handle_command(input).await;
                match result {
                    Ok(CommandResult::Exit) => break,
                    Ok(CommandResult::Message(msg)) => {
                        self.output.write(OutputEvent::Text(msg));
                        self.output.write(OutputEvent::NewLine);
                    }
                    Ok(CommandResult::SwitchMode(new_mode)) => {
                        self.mode = new_mode;
                        self.output.write(OutputEvent::Status(format!(
                            "Switched to {} mode",
                            self.mode.name()
                        )));
                        self.output.write(OutputEvent::NewLine);
                    }
                    Ok(CommandResult::Clear) => {
                        // Already handled by command
                        self.output.write(OutputEvent::NewLine);
                    }
                    Ok(CommandResult::Ok) => {
                        self.output.write(OutputEvent::NewLine);
                    }
                    Ok(CommandResult::Continue) => {
                        // Should not happen, but treat as regular input
                        self.handle_chat(input).await;
                    }
                    Err(e) => {
                        self.output.write(OutputEvent::Error(format!(
                            "Command error: {}",
                            e
                        )));
                        self.output.write(OutputEvent::NewLine);
                    }
                }
                continue;
            }

            // Regular chat input
            self.handle_chat(input).await;
        }

        Ok(())
    }

    /// Handle a slash command
    async fn handle_command(&mut self, input: &str) -> Result<CommandResult> {
        let server_filter_refs: Option<Vec<String>> = self.server_filter.clone();

        let mut ctx = CommandContext {
            agent: self.agent,
            output: self.output,
            mode: &self.mode,
            server_filter: server_filter_refs.as_ref().map(|v| v.as_slice()),
        };

        match self.command_registry.try_execute(input, &mut ctx).await {
            Some(result) => result,
            None => {
                // Not a command, treat as chat
                Ok(CommandResult::Continue)
            }
        }
    }

    /// Handle regular chat input
    async fn handle_chat(&mut self, input: &str) {
        // Apply mode-specific system prompt modifier
        if let Some(modifier) = self.mode.system_prompt_modifier() {
            // Temporarily append to system prompt
            let original_prompt = self.agent.system_prompt().map(|s| s.to_string());
            if let Some(ref base) = original_prompt {
                self.agent.set_system_prompt(Some(format!("{}{}", base, modifier)));
            } else {
                self.agent.set_system_prompt(Some(modifier));
            }
        }

        // Execute chat
        let result = if let Some(ref servers) = self.server_filter {
            let server_refs: Vec<&str> = servers.iter().map(|s| s.as_str()).collect();
            self.agent.chat_with_servers(input, &server_refs).await
        } else {
            self.agent.chat(input).await
        };

        match result {
            Ok(response) => {
                self.output.write(OutputEvent::NewLine);
                self.output.write(OutputEvent::Text(response));
                self.output.write(OutputEvent::NewLine);
            }
            Err(e) => {
                self.output.write(OutputEvent::Error(format!("{}", e)));
                self.output.write(OutputEvent::NewLine);
            }
        }
    }

    /// Get current mode
    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    /// Set mode
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_config_default() {
        let config = ReplConfig::default();
        assert!(config.server_filter.is_none());
        assert!(config.initial_mode.is_normal());
    }
}
