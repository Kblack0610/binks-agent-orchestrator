//! Implement command - enter implementation mode

use super::{CommandContext, CommandResult, SlashCommand};
use crate::cli::modes::Mode;
use crate::output::OutputEvent;
use anyhow::Result;
use async_trait::async_trait;

/// Implement command - switch to implementation mode
pub struct ImplementCommand;

#[async_trait]
impl SlashCommand for ImplementCommand {
    fn name(&self) -> &'static str {
        "implement"
    }

    fn description(&self) -> &'static str {
        "Enter implementation mode (focus on code)"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["impl", "i"]
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext<'_>) -> Result<CommandResult> {
        let args = args.trim();

        // If already in implement mode, show status
        if let Mode::Implement {
            plan,
            files_modified,
        } = ctx.mode
        {
            if args.is_empty() {
                let mut output = String::new();
                output.push_str("Currently in implementation mode\n\n");

                if let Some(p) = plan {
                    output.push_str(&format!("Plan reference: {}\n", p));
                }

                if files_modified.is_empty() {
                    output.push_str("No files modified yet.\n");
                } else {
                    output.push_str("\nFiles modified:\n");
                    for file in files_modified {
                        output.push_str(&format!("  - {}\n", file));
                    }
                }

                output.push_str("\nUse /normal to exit implementation mode.\n");
                ctx.output.write(OutputEvent::Text(output));
                return Ok(CommandResult::Ok);
            }

            // Update with new plan reference
            ctx.output.write(OutputEvent::Text(format!(
                "Setting plan reference: {}\n",
                args
            )));

            return Ok(CommandResult::SwitchMode(Mode::Implement {
                plan: Some(args.to_string()),
                files_modified: files_modified.clone(),
            }));
        }

        // Coming from plan mode - carry over context
        let plan_ref = if let Mode::Plan { context, steps } = ctx.mode {
            // Build plan summary
            let mut summary = context.clone();
            if !steps.is_empty() {
                summary.push_str("\n\nPlan steps:\n");
                for (i, step) in steps.iter().enumerate() {
                    summary.push_str(&format!("{}. {}\n", i + 1, step));
                }
            }
            Some(summary)
        } else if !args.is_empty() {
            Some(args.to_string())
        } else {
            None
        };

        let plan_msg = plan_ref
            .as_ref()
            .map(|p| format!(" Plan: {}", p.lines().next().unwrap_or("(none)")))
            .unwrap_or_default();

        ctx.output.write(OutputEvent::System(format!(
            "Entering implementation mode.{}",
            plan_msg
        )));
        ctx.output.write(OutputEvent::System(
            "Focus on writing code. Use /normal when done.".to_string(),
        ));

        Ok(CommandResult::SwitchMode(Mode::Implement {
            plan: plan_ref,
            files_modified: vec![],
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implement_metadata() {
        let cmd = ImplementCommand;
        assert_eq!(cmd.name(), "implement");
        assert!(!cmd.description().is_empty());
        assert!(cmd.aliases().contains(&"impl"));
        assert!(cmd.aliases().contains(&"i"));
    }
}
