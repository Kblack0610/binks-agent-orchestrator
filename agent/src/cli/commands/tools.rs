//! Tools command - list available tools and servers

use super::{CommandContext, CommandResult, SlashCommand};
use crate::output::OutputEvent;
use anyhow::Result;
use async_trait::async_trait;

/// Tools command
pub struct ToolsCommand;

#[async_trait]
impl SlashCommand for ToolsCommand {
    fn name(&self) -> &'static str {
        "tools"
    }

    fn description(&self) -> &'static str {
        "List available tools and servers"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["t", "servers"]
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext<'_>) -> Result<CommandResult> {
        let args = args.trim();

        match args {
            "" | "list" => {
                // List all tools
                match ctx.agent.tool_names().await {
                    Ok(names) => {
                        let mut output = String::new();
                        output.push_str(&format!("Available tools ({}):\n\n", names.len()));

                        for name in names {
                            output.push_str(&format!("  {}\n", name));
                        }

                        ctx.output.write(OutputEvent::Text(output));
                    }
                    Err(e) => {
                        ctx.output.write(OutputEvent::Error(format!(
                            "Error listing tools: {}",
                            e
                        )));
                    }
                }
            }

            "servers" => {
                // List servers
                match ctx.agent.server_names().await {
                    Ok(names) => {
                        let mut output = String::new();
                        output.push_str(&format!("Available MCP servers ({}):\n\n", names.len()));

                        for name in names {
                            output.push_str(&format!("  {}\n", name));
                        }

                        ctx.output.write(OutputEvent::Text(output));
                    }
                    Err(e) => {
                        ctx.output.write(OutputEvent::Error(format!(
                            "Error listing servers: {}",
                            e
                        )));
                    }
                }
            }

            server_name => {
                // List tools for a specific server
                match ctx.agent.tools_for_server(server_name).await {
                    Ok(tools) => {
                        let mut output = String::new();
                        output.push_str(&format!(
                            "Tools for server '{}' ({}):\n\n",
                            server_name,
                            tools.len()
                        ));

                        for tool in tools {
                            output.push_str(&format!("  {}\n", tool.name));
                            if let Some(ref desc) = tool.description {
                                // Show first line of description
                                if let Some(first_line) = desc.lines().next() {
                                    output.push_str(&format!("    {}\n", first_line));
                                }
                            }
                        }

                        ctx.output.write(OutputEvent::Text(output));
                    }
                    Err(e) => {
                        ctx.output.write(OutputEvent::Error(format!(
                            "Error listing tools for '{}': {}",
                            server_name, e
                        )));
                    }
                }
            }
        }

        Ok(CommandResult::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_metadata() {
        let cmd = ToolsCommand;
        assert_eq!(cmd.name(), "tools");
        assert!(!cmd.description().is_empty());
        assert!(cmd.aliases().contains(&"t"));
    }
}
