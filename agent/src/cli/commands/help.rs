//! Help command - displays available commands

use super::{CommandContext, CommandResult, SlashCommand};
use anyhow::Result;
use async_trait::async_trait;

/// Help command
pub struct HelpCommand;

#[async_trait]
impl SlashCommand for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        "Show available commands"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["h", "?"]
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext<'_>) -> Result<CommandResult> {
        use crate::cli::commands::CommandRegistry;
        use crate::output::OutputEvent;

        let registry = CommandRegistry::new();
        let commands = registry.available_commands(ctx.mode);

        let mut help_text = String::new();
        help_text.push_str("Available commands:\n\n");

        for cmd in commands {
            let aliases = cmd.aliases();
            let alias_str = if aliases.is_empty() {
                String::new()
            } else {
                format!(" ({})", aliases.iter().map(|a| format!("/{}", a)).collect::<Vec<_>>().join(", "))
            };

            help_text.push_str(&format!(
                "  /{:<12} {}{}\n",
                cmd.name(),
                cmd.description(),
                alias_str
            ));
        }

        help_text.push_str("\nBuilt-in commands:\n");
        help_text.push_str("  quit, exit   Exit the agent\n");

        ctx.output.write(OutputEvent::Text(help_text));

        Ok(CommandResult::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_metadata() {
        let cmd = HelpCommand;
        assert_eq!(cmd.name(), "help");
        assert!(!cmd.description().is_empty());
        assert!(cmd.aliases().contains(&"h"));
    }
}
