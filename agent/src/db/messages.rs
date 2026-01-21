//! Message CRUD operations

use super::Database;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            "system" => Ok(MessageRole::System),
            "tool" => Ok(MessageRole::Tool),
            _ => anyhow::bail!("Unknown message role: {}", s),
        }
    }
}

/// Tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Tool result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

/// Message record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_results: Option<Vec<ToolResult>>,
    pub created_at: DateTime<Utc>,
}

/// Parameters for creating a new message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessage {
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_results: Option<Vec<ToolResult>>,
}

impl Database {
    /// Create a new message
    pub fn create_message(&self, params: CreateMessage) -> Result<Message> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let tool_calls_json = params
            .tool_calls
            .as_ref()
            .map(|tc| serde_json::to_string(tc).unwrap());
        let tool_results_json = params
            .tool_results
            .as_ref()
            .map(|tr| serde_json::to_string(tr).unwrap());

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO messages (id, conversation_id, role, content, tool_calls, tool_results, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            (
                &id,
                &params.conversation_id,
                params.role.to_string(),
                &params.content,
                &tool_calls_json,
                &tool_results_json,
                now.to_rfc3339(),
            ),
        )
        .context("Failed to create message")?;

        // Update conversation's updated_at
        conn.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            (now.to_rfc3339(), &params.conversation_id),
        )?;

        Ok(Message {
            id,
            conversation_id: params.conversation_id,
            role: params.role,
            content: params.content,
            tool_calls: params.tool_calls,
            tool_results: params.tool_results,
            created_at: now,
        })
    }

    /// Get all messages for a conversation
    pub fn get_messages(&self, conversation_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, conversation_id, role, content, tool_calls, tool_results, created_at
            FROM messages
            WHERE conversation_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;

        let messages = stmt
            .query_map([conversation_id], |row| {
                let role_str: String = row.get(2)?;
                let created_at: String = row.get(6)?;
                let tool_calls_str: Option<String> = row.get(4)?;
                let tool_results_str: Option<String> = row.get(5)?;

                Ok(Message {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    role: role_str.parse().unwrap_or(MessageRole::User),
                    content: row.get(3)?,
                    tool_calls: tool_calls_str.and_then(|s| serde_json::from_str(&s).ok()),
                    tool_results: tool_results_str.and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    /// Get the last N messages for a conversation
    pub fn get_recent_messages(&self, conversation_id: &str, limit: u32) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, conversation_id, role, content, tool_calls, tool_results, created_at
            FROM messages
            WHERE conversation_id = ?1
            ORDER BY created_at DESC
            LIMIT ?2
            "#,
        )?;

        let mut messages: Vec<Message> = stmt
            .query_map([conversation_id, &limit.to_string()], |row| {
                let role_str: String = row.get(2)?;
                let created_at: String = row.get(6)?;
                let tool_calls_str: Option<String> = row.get(4)?;
                let tool_results_str: Option<String> = row.get(5)?;

                Ok(Message {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    role: role_str.parse().unwrap_or(MessageRole::User),
                    content: row.get(3)?,
                    tool_calls: tool_calls_str.and_then(|s| serde_json::from_str(&s).ok()),
                    tool_results: tool_results_str.and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Reverse to get chronological order
        messages.reverse();
        Ok(messages)
    }

    /// Delete a specific message
    pub fn delete_message(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM messages WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Count messages in a conversation
    pub fn count_messages(&self, conversation_id: &str) -> Result<u32> {
        let conn = self.conn.lock().unwrap();
        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = ?1",
            [conversation_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::conversations::CreateConversation;
    use tempfile::{tempdir, TempDir};

    fn test_db() -> (Database, TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open_at(path).unwrap();
        (db, dir)
    }

    #[test]
    fn test_create_message() {
        let (db, _dir) = test_db();
        let conv = db
            .create_conversation(CreateConversation {
                title: Some("Test".to_string()),
                system_prompt: None,
                metadata: None,
            })
            .unwrap();

        let msg = db
            .create_message(CreateMessage {
                conversation_id: conv.id.clone(),
                role: MessageRole::User,
                content: "Hello!".to_string(),
                tool_calls: None,
                tool_results: None,
            })
            .unwrap();

        assert_eq!(msg.content, "Hello!");
        assert_eq!(msg.role, MessageRole::User);
    }

    #[test]
    fn test_get_messages() {
        let (db, _dir) = test_db();
        let conv = db
            .create_conversation(CreateConversation {
                title: Some("Test".to_string()),
                system_prompt: None,
                metadata: None,
            })
            .unwrap();

        // Create multiple messages
        for i in 0..5 {
            db.create_message(CreateMessage {
                conversation_id: conv.id.clone(),
                role: if i % 2 == 0 {
                    MessageRole::User
                } else {
                    MessageRole::Assistant
                },
                content: format!("Message {}", i),
                tool_calls: None,
                tool_results: None,
            })
            .unwrap();
        }

        let messages = db.get_messages(&conv.id).unwrap();
        assert_eq!(messages.len(), 5);
    }

    #[test]
    fn test_message_with_tool_calls() {
        let (db, _dir) = test_db();
        let conv = db
            .create_conversation(CreateConversation {
                title: Some("Test".to_string()),
                system_prompt: None,
                metadata: None,
            })
            .unwrap();

        let tool_calls = vec![ToolCall {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            arguments: serde_json::json!({"location": "NYC"}),
        }];

        let msg = db
            .create_message(CreateMessage {
                conversation_id: conv.id.clone(),
                role: MessageRole::Assistant,
                content: "Let me check the weather.".to_string(),
                tool_calls: Some(tool_calls.clone()),
                tool_results: None,
            })
            .unwrap();

        assert!(msg.tool_calls.is_some());
        assert_eq!(msg.tool_calls.unwrap()[0].name, "get_weather");
    }

    #[test]
    fn test_cascade_delete() {
        let (db, _dir) = test_db();
        let conv = db
            .create_conversation(CreateConversation {
                title: Some("Test".to_string()),
                system_prompt: None,
                metadata: None,
            })
            .unwrap();

        db.create_message(CreateMessage {
            conversation_id: conv.id.clone(),
            role: MessageRole::User,
            content: "Hello!".to_string(),
            tool_calls: None,
            tool_results: None,
        })
        .unwrap();

        // Delete conversation should cascade to messages
        db.delete_conversation(&conv.id).unwrap();
        let messages = db.get_messages(&conv.id).unwrap();
        assert_eq!(messages.len(), 0);
    }
}
