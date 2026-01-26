//! Normal command - return to normal mode

use super::{CommandContext, CommandResult, SlashCommand};
use crate::cli::modes::Mode;
use crate::output::OutputEvent;
use anyhow::Result;
use async_trait::async_trait;

/// Normal command - switch back to normal mode
pub struct NormalCommand;

#[async_trait]
impl SlashCommand for NormalCommand {
    fn name(&self) -> &'static str {
        "normal"
    }

    fn description(&self) -> &'static str {
        "Return to normal conversation mode"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["n", "exit"]
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext<'_>) -> Result<CommandResult> {
        // If already in normal mode, just confirm
        if ctx.mode.is_normal() {
            ctx.output
                .write(OutputEvent::Text("Already in normal mode.\n".to_string()));
            return Ok(CommandResult::Ok);
        }

        // Show summary of what was done in the previous mode
        let summary = match ctx.mode {
            Mode::Plan { context, steps } => {
                let mut msg = format!("Exiting plan mode. Context: {}", context);
                if !steps.is_empty() {
                    msg.push_str(&format!("\n  {} steps recorded.", steps.len()));
                }
                msg
            }
            Mode::Implement {
                plan,
                files_modified,
            } => {
                let mut msg = "Exiting implementation mode.".to_string();
                if plan.is_some() {
                    msg.push_str(" Had plan reference.");
                }
                if !files_modified.is_empty() {
                    msg.push_str(&format!("\n  {} files modified.", files_modified.len()));
                }
                msg
            }
            Mode::Normal => "Already in normal mode.".to_string(),
        };

        ctx.output.write(OutputEvent::System(summary));

        Ok(CommandResult::SwitchMode(Mode::Normal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_metadata() {
        let cmd = NormalCommand;
        assert_eq!(cmd.name(), "normal");
        assert!(!cmd.description().is_empty());
        assert!(cmd.aliases().contains(&"n"));
        assert!(cmd.aliases().contains(&"exit"));
    }
}
