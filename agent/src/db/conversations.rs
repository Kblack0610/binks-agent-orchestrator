//! Conversation CRUD operations

use super::Database;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Conversation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub system_prompt: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Parameters for creating a new conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversation {
    pub title: Option<String>,
    pub system_prompt: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Parameters for updating a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConversation {
    pub title: Option<String>,
    pub system_prompt: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl Database {
    /// Create a new conversation
    pub fn create_conversation(&self, params: CreateConversation) -> Result<Conversation> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let title = params
            .title
            .unwrap_or_else(|| "New Conversation".to_string());
        let metadata_json = params
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap());

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO conversations (id, title, created_at, updated_at, system_prompt, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            (
                &id,
                &title,
                now.to_rfc3339(),
                now.to_rfc3339(),
                &params.system_prompt,
                &metadata_json,
            ),
        )
        .context("Failed to create conversation")?;

        Ok(Conversation {
            id,
            title,
            created_at: now,
            updated_at: now,
            system_prompt: params.system_prompt,
            metadata: params.metadata,
        })
    }

    /// Get a conversation by ID
    pub fn get_conversation(&self, id: &str) -> Result<Option<Conversation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, title, created_at, updated_at, system_prompt, metadata
            FROM conversations
            WHERE id = ?1
            "#,
        )?;

        let result = stmt.query_row([id], |row| {
            let created_at: String = row.get(2)?;
            let updated_at: String = row.get(3)?;
            let metadata_str: Option<String> = row.get(5)?;

            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .unwrap()
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at)
                    .unwrap()
                    .with_timezone(&Utc),
                system_prompt: row.get(4)?,
                metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
            })
        });

        match result {
            Ok(conv) => Ok(Some(conv)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all conversations, ordered by most recently updated
    pub fn list_conversations(
        &self,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Conversation>> {
        let conn = self.conn.lock().unwrap();
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        let mut stmt = conn.prepare(
            r#"
            SELECT id, title, created_at, updated_at, system_prompt, metadata
            FROM conversations
            ORDER BY updated_at DESC
            LIMIT ?1 OFFSET ?2
            "#,
        )?;

        let conversations = stmt
            .query_map([limit, offset], |row| {
                let created_at: String = row.get(2)?;
                let updated_at: String = row.get(3)?;
                let metadata_str: Option<String> = row.get(5)?;

                Ok(Conversation {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at)
                        .unwrap()
                        .with_timezone(&Utc),
                    system_prompt: row.get(4)?,
                    metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(conversations)
    }

    /// Update a conversation
    pub fn update_conversation(
        &self,
        id: &str,
        params: UpdateConversation,
    ) -> Result<Option<Conversation>> {
        let now = Utc::now();
        let metadata_json = params
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap());

        let conn = self.conn.lock().unwrap();

        // Simple approach: update each field if present using COALESCE
        conn.execute(
            "UPDATE conversations SET updated_at = ?1, title = COALESCE(?2, title), system_prompt = COALESCE(?3, system_prompt), metadata = COALESCE(?4, metadata) WHERE id = ?5",
            (
                now.to_rfc3339(),
                &params.title,
                &params.system_prompt,
                &metadata_json,
                id,
            ),
        )?;

        drop(conn);
        self.get_conversation(id)
    }

    /// Delete a conversation (cascades to messages)
    pub fn delete_conversation(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM conversations WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    /// Touch a conversation (update the updated_at timestamp)
    pub fn touch_conversation(&self, id: &str) -> Result<()> {
        let now = Utc::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            (now.to_rfc3339(), id),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    fn test_db() -> (Database, TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open_at(path).unwrap();
        (db, dir)
    }

    #[test]
    fn test_create_conversation() {
        let (db, _dir) = test_db();
        let conv = db
            .create_conversation(CreateConversation {
                title: Some("Test Conversation".to_string()),
                system_prompt: Some("You are helpful".to_string()),
                metadata: None,
            })
            .unwrap();

        assert_eq!(conv.title, "Test Conversation");
        assert_eq!(conv.system_prompt, Some("You are helpful".to_string()));
    }

    #[test]
    fn test_get_conversation() {
        let (db, _dir) = test_db();
        let created = db
            .create_conversation(CreateConversation {
                title: Some("Test".to_string()),
                system_prompt: None,
                metadata: None,
            })
            .unwrap();

        let fetched = db.get_conversation(&created.id).unwrap().unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.title, created.title);
    }

    #[test]
    fn test_list_conversations() {
        let (db, _dir) = test_db();

        for i in 0..5 {
            db.create_conversation(CreateConversation {
                title: Some(format!("Conversation {}", i)),
                system_prompt: None,
                metadata: None,
            })
            .unwrap();
        }

        let list = db.list_conversations(Some(10), None).unwrap();
        assert_eq!(list.len(), 5);
    }

    #[test]
    fn test_delete_conversation() {
        let (db, _dir) = test_db();
        let conv = db
            .create_conversation(CreateConversation {
                title: Some("To Delete".to_string()),
                system_prompt: None,
                metadata: None,
            })
            .unwrap();

        assert!(db.delete_conversation(&conv.id).unwrap());
        assert!(db.get_conversation(&conv.id).unwrap().is_none());
    }
}
