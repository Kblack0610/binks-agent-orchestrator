//! MCP Server implementation for local inbox

use chrono::{Local, NaiveDate};
use mcp_common::{internal_error, json_success, CallToolResult, McpError};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::types::{InboxMessage, Priority, ReadResponse, WriteResponse};

/// The main Inbox MCP Server
#[derive(Clone)]
pub struct InboxMcpServer {
    inbox_path: PathBuf,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WriteInboxParams {
    #[schemars(description = "The message content to write to inbox")]
    pub message: String,

    #[schemars(description = "Source of the message (e.g., 'monitor', 'task', 'github')")]
    #[serde(default = "default_source")]
    pub source: String,

    #[schemars(description = "Priority level: 'low', 'normal', 'high', or 'urgent'")]
    #[serde(default)]
    pub priority: Option<String>,

    #[schemars(description = "Tags for categorization (e.g., ['pr', 'review'])")]
    #[serde(default)]
    pub tags: Vec<String>,

    #[schemars(description = "Optional URL reference")]
    pub url: Option<String>,
}

fn default_source() -> String {
    "agent".to_string()
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadInboxParams {
    #[schemars(description = "Number of days to look back (default: 1)")]
    pub days: Option<u32>,

    #[schemars(description = "Filter by source")]
    pub source: Option<String>,

    #[schemars(description = "Filter by tag")]
    pub tag: Option<String>,

    #[schemars(description = "Maximum number of messages to return")]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ClearInboxParams {
    #[schemars(description = "Number of days to keep (archive older messages)")]
    pub keep_days: Option<u32>,

    #[schemars(description = "Actually delete instead of archive")]
    #[serde(default)]
    pub delete: bool,
}

// ============================================================================
// Tool Router Implementation
// ============================================================================

#[tool_router]
impl InboxMcpServer {
    pub fn new() -> Self {
        // Get inbox path from env or default to ~/.notes/inbox
        let inbox_path = std::env::var("INBOX_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".notes")
                    .join("inbox")
            });

        Self {
            inbox_path,
            tool_router: Self::tool_router(),
        }
    }

    /// Get the file path for a given date
    fn get_file_path(&self, date: NaiveDate) -> PathBuf {
        self.inbox_path
            .join(format!("{}.md", date.format("%Y-%m-%d")))
    }

    /// Ensure the inbox directory exists
    async fn ensure_inbox_dir(&self) -> Result<(), McpError> {
        fs::create_dir_all(&self.inbox_path)
            .await
            .map_err(|e| internal_error(format!("Failed to create inbox directory: {e}")))
    }

    // ========================================================================
    // Write Tool
    // ========================================================================

    #[tool(
        description = "Write a message to the local inbox. Messages are stored in ~/.notes/inbox/YYYY-MM-DD.md files with timestamp, source, priority, and tags."
    )]
    async fn write_inbox(
        &self,
        Parameters(params): Parameters<WriteInboxParams>,
    ) -> Result<CallToolResult, McpError> {
        self.ensure_inbox_dir().await?;

        let now = Local::now();
        let file_path = self.get_file_path(now.date_naive());

        // Parse priority
        let priority = match params.priority.as_deref() {
            Some("low") => Priority::Low,
            Some("high") => Priority::High,
            Some("urgent") => Priority::Urgent,
            _ => Priority::Normal,
        };

        // Create the message
        let message = InboxMessage {
            timestamp: now,
            source: params.source,
            priority,
            tags: params.tags,
            message: params.message,
            url: params.url,
        };

        // Format as markdown
        let markdown = message.to_markdown();

        // Append to file (create if doesn't exist)
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await
            .map_err(|e| internal_error(format!("Failed to open inbox file: {e}")))?;

        // Add separator if file is not empty
        let metadata = file
            .metadata()
            .await
            .map_err(|e| internal_error(format!("Failed to get file metadata: {e}")))?;

        let content = if metadata.len() > 0 {
            format!("\n---\n\n{}\n", markdown)
        } else {
            format!("# Inbox - {}\n\n{}\n", now.format("%Y-%m-%d"), markdown)
        };

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| internal_error(format!("Failed to write to inbox: {e}")))?;

        let response = WriteResponse {
            success: true,
            file_path: file_path.to_string_lossy().to_string(),
            message_id: format!(
                "{}_{}",
                now.format("%Y%m%d%H%M%S"),
                now.timestamp_subsec_millis()
            ),
        };

        json_success(&response)
    }

    // ========================================================================
    // Read Tool
    // ========================================================================

    #[tool(
        description = "Read messages from the local inbox. Can filter by date range, source, and tags."
    )]
    async fn read_inbox(
        &self,
        Parameters(params): Parameters<ReadInboxParams>,
    ) -> Result<CallToolResult, McpError> {
        let days = params.days.unwrap_or(1);
        let today = Local::now().date_naive();

        let mut all_messages = Vec::new();
        let mut files_read = Vec::new();

        // Read files for each day
        for day_offset in 0..days {
            let date = today - chrono::Duration::days(day_offset as i64);
            let file_path = self.get_file_path(date);

            if file_path.exists() {
                files_read.push(file_path.to_string_lossy().to_string());

                let content = fs::read_to_string(&file_path)
                    .await
                    .map_err(|e| internal_error(format!("Failed to read inbox file: {e}")))?;

                // Parse messages from markdown (simple parsing)
                // Messages start with "## YYYY-MM-DD HH:MM:SS"
                for section in content.split("\n---\n") {
                    if let Some(msg) = self.parse_message_from_markdown(section) {
                        // Apply filters
                        if let Some(ref source) = params.source {
                            if msg.source != *source {
                                continue;
                            }
                        }
                        if let Some(ref tag) = params.tag {
                            if !msg.tags.contains(tag) {
                                continue;
                            }
                        }
                        all_messages.push(msg);
                    }
                }
            }
        }

        // Sort by timestamp descending (newest first)
        all_messages.sort_by_key(|m| std::cmp::Reverse(m.timestamp));

        // Apply limit
        let total_count = all_messages.len();
        if let Some(limit) = params.limit {
            all_messages.truncate(limit);
        }

        let response = ReadResponse {
            messages: all_messages,
            total_count,
            files_read,
        };

        json_success(&response)
    }

    /// Parse an inbox message from markdown section
    fn parse_message_from_markdown(&self, section: &str) -> Option<InboxMessage> {
        let lines: Vec<&str> = section.lines().collect();

        // Find the header line starting with "## "
        let header_idx = lines.iter().position(|l| l.starts_with("## "))?;
        let header = lines[header_idx];

        // Parse header: "## 2026-01-17 14:30:00 [source] #tag1 #tag2 *[priority]*"
        let header = header.strip_prefix("## ")?;

        // Extract timestamp (first 19 chars: YYYY-MM-DD HH:MM:SS)
        if header.len() < 19 {
            return None;
        }
        let timestamp_str = &header[..19];
        let timestamp =
            chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S").ok()?;
        let timestamp = timestamp.and_local_timezone(Local).single()?;

        let rest = &header[19..].trim();

        // Extract source from [source]
        let source_start = rest.find('[')?;
        let source_end = rest.find(']')?;
        let source = rest[source_start + 1..source_end].to_string();

        let rest = &rest[source_end + 1..];

        // Extract tags (#tag)
        let tags: Vec<String> = rest
            .split_whitespace()
            .filter(|w| w.starts_with('#') && !w.contains('['))
            .map(|w| w.trim_start_matches('#').to_string())
            .collect();

        // Extract priority
        let priority = if rest.contains("**[URGENT]**") {
            Priority::Urgent
        } else if rest.contains("*[HIGH]*") {
            Priority::High
        } else if rest.contains("[LOW]") {
            Priority::Low
        } else {
            Priority::Normal
        };

        // Message content is everything after the header
        let message_lines: Vec<&str> = lines[header_idx + 1..]
            .iter()
            .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("---"))
            .copied()
            .collect();

        // Check if last line is a URL
        let (message, url) = if let Some(last) = message_lines.last() {
            if last.starts_with("http://") || last.starts_with("https://") {
                let msg = message_lines[..message_lines.len() - 1].join("\n");
                (msg, Some(last.to_string()))
            } else {
                (message_lines.join("\n"), None)
            }
        } else {
            (String::new(), None)
        };

        Some(InboxMessage {
            timestamp,
            source,
            priority,
            tags,
            message,
            url,
        })
    }

    // ========================================================================
    // Clear Tool
    // ========================================================================

    #[tool(
        description = "Archive or delete old inbox messages. By default, archives messages older than the specified days."
    )]
    async fn clear_inbox(
        &self,
        Parameters(params): Parameters<ClearInboxParams>,
    ) -> Result<CallToolResult, McpError> {
        let keep_days = params.keep_days.unwrap_or(7);
        let today = Local::now().date_naive();
        let cutoff = today - chrono::Duration::days(keep_days as i64);

        let mut archived_count = 0;
        let archive_path = if !params.delete {
            Some(self.inbox_path.join("archive"))
        } else {
            None
        };

        // Create archive directory if needed
        if let Some(ref archive) = archive_path {
            fs::create_dir_all(archive)
                .await
                .map_err(|e| internal_error(format!("Failed to create archive directory: {e}")))?;
        }

        // List all .md files in inbox
        let mut entries = fs::read_dir(&self.inbox_path)
            .await
            .map_err(|e| internal_error(format!("Failed to read inbox directory: {e}")))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| internal_error(format!("Failed to read directory entry: {e}")))?
        {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                // Parse date from filename (YYYY-MM-DD.md)
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(file_date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") {
                        if file_date < cutoff {
                            if let Some(ref archive) = archive_path {
                                // Move to archive
                                let dest = archive.join(path.file_name().unwrap());
                                fs::rename(&path, dest).await.map_err(|e| {
                                    internal_error(format!("Failed to archive file: {e}"))
                                })?;
                            } else {
                                // Delete
                                fs::remove_file(&path).await.map_err(|e| {
                                    internal_error(format!("Failed to delete file: {e}"))
                                })?;
                            }
                            archived_count += 1;
                        }
                    }
                }
            }
        }

        let response = crate::types::ClearResponse {
            archived_count,
            archive_path: archive_path.map(|p| p.to_string_lossy().to_string()),
        };

        json_success(&response)
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for InboxMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Local file-based inbox MCP server for agent notifications. \
                 Messages are stored in ~/.notes/inbox/YYYY-MM-DD.md files \
                 with timestamps, sources, priorities, and tags."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for InboxMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
