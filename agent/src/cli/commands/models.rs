//! Models command - list and switch models

use super::{CommandContext, CommandResult, SlashCommand};
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

            // Try to fetch available models from Ollama
            match list_ollama_models(ctx.agent.ollama_url()).await {
                Ok(models) => {
                    let mut output = String::new();
                    output.push_str("Available models:\n\n");

                    for model in models {
                        let marker = if model == current { " *" } else { "" };
                        output.push_str(&format!("  {}{}\n", model, marker));
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
                    ctx.output.write(OutputEvent::Text(format!(
                        "Current model: {}\n",
                        current
                    )));
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

/// Fetch list of models from Ollama
async fn list_ollama_models(ollama_url: &str) -> Result<Vec<String>> {
    let url = format!("{}/api/tags", ollama_url);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Ollama API error: {}", response.status());
    }

    #[derive(serde::Deserialize)]
    struct TagsResponse {
        models: Vec<ModelInfo>,
    }

    #[derive(serde::Deserialize)]
    struct ModelInfo {
        name: String,
    }

    let tags: TagsResponse = response.json().await?;
    let models: Vec<String> = tags.models.into_iter().map(|m| m.name).collect();

    Ok(models)
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
