//! Clear command - clear history and/or screen

use super::{CommandContext, CommandResult, SlashCommand};
use crate::output::OutputEvent;
use anyhow::Result;
use async_trait::async_trait;

/// Clear command
pub struct ClearCommand;

#[async_trait]
impl SlashCommand for ClearCommand {
    fn name(&self) -> &'static str {
        "clear"
    }

    fn description(&self) -> &'static str {
        "Clear conversation history"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["c", "reset"]
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext<'_>) -> Result<CommandResult> {
        let args = args.trim();

        match args {
            "" | "history" => {
                // Clear history
                ctx.agent.clear_history();
                ctx.output.write(OutputEvent::Status(
                    "Conversation history cleared".to_string(),
                ));
                Ok(CommandResult::Clear)
            }

            "screen" => {
                // Clear screen (ANSI escape)
                print!("\x1B[2J\x1B[H");
                Ok(CommandResult::Ok)
            }

            "all" => {
                // Clear both
                ctx.agent.clear_history();
                print!("\x1B[2J\x1B[H");
                ctx.output.write(OutputEvent::Status(
                    "History and screen cleared".to_string(),
                ));
                Ok(CommandResult::Clear)
            }

            _ => {
                ctx.output.write(OutputEvent::Warning(
                    "Usage: /clear [history|screen|all]".to_string(),
                ));
                Ok(CommandResult::Ok)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_metadata() {
        let cmd = ClearCommand;
        assert_eq!(cmd.name(), "clear");
        assert!(!cmd.description().is_empty());
        assert!(cmd.aliases().contains(&"c"));
    }
}
