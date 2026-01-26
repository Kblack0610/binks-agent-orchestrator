//! Plan command - enter planning mode

use super::{CommandContext, CommandResult, SlashCommand};
use crate::cli::modes::Mode;
use crate::output::OutputEvent;
use anyhow::Result;
use async_trait::async_trait;

/// Plan command - switch to planning mode
pub struct PlanCommand;

#[async_trait]
impl SlashCommand for PlanCommand {
    fn name(&self) -> &'static str {
        "plan"
    }

    fn description(&self) -> &'static str {
        "Enter planning mode (focus on analysis)"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["p"]
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext<'_>) -> Result<CommandResult> {
        let args = args.trim();

        // If already in plan mode, show status or add step
        if let Mode::Plan { context, steps } = ctx.mode {
            if args.is_empty() {
                // Show current plan status
                let mut output = String::new();
                output.push_str("Currently in planning mode\n\n");
                output.push_str(&format!("Context: {}\n", context));

                if steps.is_empty() {
                    output.push_str("No steps recorded yet.\n");
                } else {
                    output.push_str("\nSteps:\n");
                    for (i, step) in steps.iter().enumerate() {
                        output.push_str(&format!("  {}. {}\n", i + 1, step));
                    }
                }

                output.push_str(
                    "\nUse /implement to start implementing, /normal to exit plan mode.\n",
                );
                ctx.output.write(OutputEvent::Text(output));
                return Ok(CommandResult::Ok);
            }

            // Otherwise treat as adding context/info
            ctx.output.write(OutputEvent::Text(format!(
                "Updating plan context with: {}\n",
                args
            )));

            // Return with updated context
            return Ok(CommandResult::SwitchMode(Mode::Plan {
                context: format!("{}\n{}", context, args),
                steps: steps.clone(),
            }));
        }

        // Enter plan mode
        let context = if args.is_empty() {
            "General planning".to_string()
        } else {
            args.to_string()
        };

        ctx.output.write(OutputEvent::System(format!(
            "Entering plan mode. Context: {}",
            context
        )));
        ctx.output.write(OutputEvent::System(
            "Focus on analysis and planning. Use /implement when ready to code.".to_string(),
        ));

        Ok(CommandResult::SwitchMode(Mode::Plan {
            context,
            steps: vec![],
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_metadata() {
        let cmd = PlanCommand;
        assert_eq!(cmd.name(), "plan");
        assert!(!cmd.description().is_empty());
        assert!(cmd.aliases().contains(&"p"));
    }
}
