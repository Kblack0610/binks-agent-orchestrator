//! Models command - list and switch models

use super::{CommandContext, CommandResult, SlashCommand};
use crate::llm;
use crate::output::OutputEvent;
use anyhow::Result;
use async_trait::async_trait;

/// Models command
pub struct ModelsCommand;

#[async_trait]
impl SlashCommand for ModelsCommand {
    fn name(&self) -> &'static str {
        "models"
    }

    fn description(&self) -> &'static str {
        "List available models or switch model"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["model", "m"]
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext<'_>) -> Result<CommandResult> {
        let args = args.trim();

        if args.is_empty() {
            // List models
            let current = ctx.agent.model();

            // Try to fetch available models from the configured gateway.
            match llm::list_models(ctx.agent.gateway_url(), Some(current)).await {
                Ok(models) => {
                    let mut output = String::new();
                    output.push_str("Available models:\n\n");

                    for model in models {
                        let marker = if model.id == current { " *" } else { "" };
                        output.push_str(&format!(
                            "  {} [{}]{}\n",
                            model.display_name, model.provider, marker
                        ));
                    }

                    output.push_str(&format!("\nCurrent: {}\n", current));
                    output.push_str("\nUse /models <name> to switch models\n");

                    ctx.output.write(OutputEvent::Text(output));
                }
                Err(e) => {
                    ctx.output.write(OutputEvent::Warning(format!(
                        "Could not list models: {}",
                        e
                    )));
                    ctx.output
                        .write(OutputEvent::Text(format!("Current model: {}\n", current)));
                }
            }

            Ok(CommandResult::Ok)
        } else {
            // Switch model
            let new_model = args.to_string();
            ctx.agent.set_model(&new_model);

            Ok(CommandResult::Message(format!(
                "Switched to model: {}",
                new_model
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_models_metadata() {
        let cmd = ModelsCommand;
        assert_eq!(cmd.name(), "models");
        assert!(!cmd.description().is_empty());
        assert!(cmd.aliases().contains(&"model"));
    }
}
