//! Web command handler
//!
//! Start the web interface server.

use anyhow::Result;

use super::CommandContext;
use crate::web::{self, WebConfig};

/// Handle the `web` command
pub async fn run_web(
    ctx: &CommandContext,
    port: u16,
    system: Option<String>,
    dev: bool,
    open_browser: bool,
) -> Result<()> {
    let config = WebConfig {
        port,
        ollama_url: ctx.ollama_url.clone(),
        model: ctx.model.clone(),
        system_prompt: system.or_else(|| ctx.file_config.agent.system_prompt.clone()),
        dev_mode: dev,
    };

    if open_browser {
        let url = format!("http://localhost:{}", port);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if let Err(e) = open::that(&url) {
                tracing::warn!("Failed to open browser: {}", e);
            }
        });
    }

    web::serve(config).await
}
