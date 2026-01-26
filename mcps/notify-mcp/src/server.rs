//! MCP Server implementation for notifications

use chrono::Local;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The main Notify MCP Server
#[derive(Clone)]
pub struct NotifyMcpServer {
    slack_webhook: Option<String>,
    discord_webhook: Option<String>,
    http_client: reqwest::Client,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SlackMessageParams {
    #[schemars(description = "The message text to send")]
    pub message: String,

    #[schemars(description = "Optional channel override (if webhook supports it)")]
    pub channel: Option<String>,

    #[schemars(description = "Optional username for the bot")]
    pub username: Option<String>,

    #[schemars(description = "Optional emoji icon (e.g., ':robot:')")]
    pub icon_emoji: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiscordMessageParams {
    #[schemars(description = "The message content to send")]
    pub content: String,

    #[schemars(description = "Optional username override")]
    pub username: Option<String>,

    #[schemars(description = "Optional avatar URL")]
    pub avatar_url: Option<String>,

    #[schemars(description = "Whether this is a TTS message")]
    #[serde(default)]
    pub tts: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DigestParams {
    #[schemars(description = "Title for the digest")]
    pub title: String,

    #[schemars(description = "List of items to include in the digest")]
    pub items: Vec<DigestItem>,

    #[schemars(description = "Platforms to send to: 'slack', 'discord', or 'all'")]
    #[serde(default = "default_platform")]
    pub platform: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct DigestItem {
    #[schemars(description = "Status emoji or icon")]
    pub status: String,

    #[schemars(description = "Item description")]
    pub text: String,

    #[schemars(description = "Optional URL link")]
    pub url: Option<String>,
}

fn default_platform() -> String {
    "all".to_string()
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct NotifyResponse {
    pub success: bool,
    pub platform: String,
    pub message: String,
}

// ============================================================================
// Slack/Discord Payload Types
// ============================================================================

#[derive(Debug, Serialize)]
struct SlackPayload {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_emoji: Option<String>,
}

#[derive(Debug, Serialize)]
struct DiscordPayload {
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
    tts: bool,
}

// ============================================================================
// Tool Router Implementation
// ============================================================================

#[tool_router]
impl NotifyMcpServer {
    pub fn new() -> Self {
        let slack_webhook = std::env::var("SLACK_WEBHOOK_URL").ok();
        let discord_webhook = std::env::var("DISCORD_WEBHOOK_URL").ok();

        if slack_webhook.is_none() && discord_webhook.is_none() {
            tracing::warn!(
                "No webhook URLs configured. Set SLACK_WEBHOOK_URL or DISCORD_WEBHOOK_URL"
            );
        }

        Self {
            slack_webhook,
            discord_webhook,
            http_client: reqwest::Client::new(),
            tool_router: Self::tool_router(),
        }
    }

    // ========================================================================
    // Slack Tool
    // ========================================================================

    #[tool(
        description = "Send a message to Slack via webhook. Requires SLACK_WEBHOOK_URL environment variable."
    )]
    async fn send_slack(
        &self,
        Parameters(params): Parameters<SlackMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        let webhook_url = self.slack_webhook.as_ref().ok_or_else(|| {
            McpError::internal_error("SLACK_WEBHOOK_URL not configured".to_string(), None)
        })?;

        let payload = SlackPayload {
            text: params.message.clone(),
            channel: params.channel,
            username: params.username,
            icon_emoji: params.icon_emoji,
        };

        let response = self
            .http_client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Failed to send Slack message: {}", e), None)
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(McpError::internal_error(
                format!("Slack API error ({}): {}", status, error_text),
                None,
            ));
        }

        let result = NotifyResponse {
            success: true,
            platform: "slack".to_string(),
            message: format!(
                "Message sent successfully: {}",
                &params.message[..params.message.len().min(50)]
            ),
        };

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Discord Tool
    // ========================================================================

    #[tool(
        description = "Send a message to Discord via webhook. Requires DISCORD_WEBHOOK_URL environment variable."
    )]
    async fn send_discord(
        &self,
        Parameters(params): Parameters<DiscordMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        let webhook_url = self.discord_webhook.as_ref().ok_or_else(|| {
            McpError::internal_error("DISCORD_WEBHOOK_URL not configured".to_string(), None)
        })?;

        let payload = DiscordPayload {
            content: params.content.clone(),
            username: params.username,
            avatar_url: params.avatar_url,
            tts: params.tts,
        };

        let response = self
            .http_client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Failed to send Discord message: {}", e), None)
            })?;

        let status = response.status();
        // Discord returns 204 No Content on success
        if !status.is_success() && status.as_u16() != 204 {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(McpError::internal_error(
                format!("Discord API error ({}): {}", status, error_text),
                None,
            ));
        }

        let result = NotifyResponse {
            success: true,
            platform: "discord".to_string(),
            message: format!(
                "Message sent successfully: {}",
                &params.content[..params.content.len().min(50)]
            ),
        };

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Digest Tool
    // ========================================================================

    #[tool(
        description = "Send a formatted digest to configured notification platforms. Useful for daily summaries."
    )]
    async fn send_digest(
        &self,
        Parameters(params): Parameters<DigestParams>,
    ) -> Result<CallToolResult, McpError> {
        let now = Local::now();
        let mut results = Vec::new();

        // Format digest message
        let header = format!("📊 **{}** - {}", params.title, now.format("%Y-%m-%d %H:%M"));

        let items_text: Vec<String> = params
            .items
            .iter()
            .map(|item| {
                if let Some(ref url) = item.url {
                    format!("{} {} - {}", item.status, item.text, url)
                } else {
                    format!("{} {}", item.status, item.text)
                }
            })
            .collect();

        let full_message = format!("{}\n\n{}", header, items_text.join("\n"));

        // Send to Slack if configured and requested
        if (params.platform == "all" || params.platform == "slack") && self.slack_webhook.is_some()
        {
            let slack_params = SlackMessageParams {
                message: full_message.clone(),
                channel: None,
                username: Some("Binks Monitor".to_string()),
                icon_emoji: Some(":robot_face:".to_string()),
            };
            match self.send_slack(Parameters(slack_params)).await {
                Ok(_) => results.push("slack: success".to_string()),
                Err(e) => results.push(format!("slack: failed - {}", e)),
            }
        }

        // Send to Discord if configured and requested
        if (params.platform == "all" || params.platform == "discord")
            && self.discord_webhook.is_some()
        {
            let discord_params = DiscordMessageParams {
                content: full_message.clone(),
                username: Some("Binks Monitor".to_string()),
                avatar_url: None,
                tts: false,
            };
            match self.send_discord(Parameters(discord_params)).await {
                Ok(_) => results.push("discord: success".to_string()),
                Err(e) => results.push(format!("discord: failed - {}", e)),
            }
        }

        if results.is_empty() {
            return Err(McpError::internal_error(
                format!(
                    "No notification platforms configured for '{}'",
                    params.platform
                ),
                None,
            ));
        }

        let result = serde_json::json!({
            "success": true,
            "results": results,
            "digest_title": params.title,
            "items_count": params.items.len(),
        });

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Status Tool
    // ========================================================================

    #[tool(description = "Check which notification platforms are configured and available.")]
    async fn get_notify_status(&self) -> Result<CallToolResult, McpError> {
        let status = serde_json::json!({
            "slack": {
                "configured": self.slack_webhook.is_some(),
                "url_preview": self.slack_webhook.as_ref().map(|u| {
                    if u.len() > 20 {
                        format!("{}...", &u[..20])
                    } else {
                        u.clone()
                    }
                })
            },
            "discord": {
                "configured": self.discord_webhook.is_some(),
                "url_preview": self.discord_webhook.as_ref().map(|u| {
                    if u.len() > 20 {
                        format!("{}...", &u[..20])
                    } else {
                        u.clone()
                    }
                })
            }
        });

        let json = serde_json::to_string_pretty(&status)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for NotifyMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Notification MCP server for Slack and Discord webhooks. \
                 Configure SLACK_WEBHOOK_URL and/or DISCORD_WEBHOOK_URL \
                 environment variables to enable notifications."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for NotifyMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
