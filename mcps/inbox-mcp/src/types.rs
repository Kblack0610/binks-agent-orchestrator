//! Type definitions for inbox messages

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Priority level for inbox messages
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Normal => write!(f, "normal"),
            Priority::High => write!(f, "high"),
            Priority::Urgent => write!(f, "urgent"),
        }
    }
}

/// An inbox message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMessage {
    /// Timestamp when the message was created
    pub timestamp: DateTime<Local>,
    /// Source of the message (e.g., "monitor", "task", "github")
    pub source: String,
    /// Priority level
    pub priority: Priority,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// The message content
    pub message: String,
    /// Optional URL reference
    pub url: Option<String>,
}

impl InboxMessage {
    /// Format the message as markdown for the inbox file
    pub fn to_markdown(&self) -> String {
        let tags_str = if self.tags.is_empty() {
            String::new()
        } else {
            format!(" {}", self.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" "))
        };

        let priority_marker = match self.priority {
            Priority::Urgent => " **[URGENT]**",
            Priority::High => " *[HIGH]*",
            _ => "",
        };

        let url_line = self.url.as_ref().map(|u| format!("\n{}", u)).unwrap_or_default();

        format!(
            "## {} [{}]{}{}\n{}{}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.source,
            tags_str,
            priority_marker,
            self.message,
            url_line
        )
    }
}

/// Response when writing to inbox
#[derive(Debug, Serialize, Deserialize)]
pub struct WriteResponse {
    pub success: bool,
    pub file_path: String,
    pub message_id: String,
}

/// Response when reading from inbox
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadResponse {
    pub messages: Vec<InboxMessage>,
    pub total_count: usize,
    pub files_read: Vec<String>,
}

/// Response when clearing inbox
#[derive(Debug, Serialize, Deserialize)]
pub struct ClearResponse {
    pub archived_count: usize,
    pub archive_path: Option<String>,
}
