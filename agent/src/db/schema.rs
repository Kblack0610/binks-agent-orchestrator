//! Database schema definitions and migrations

use anyhow::Result;
use rusqlite::Connection;

/// Current schema version
pub const SCHEMA_VERSION: i32 = 1;

/// Create all tables if they don't exist
pub fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Conversations table
        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            system_prompt TEXT,
            metadata TEXT
        );

        -- Messages table
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            tool_calls TEXT,
            tool_results TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        );

        -- Index for efficient message retrieval by conversation
        CREATE INDEX IF NOT EXISTS idx_messages_conversation
        ON messages(conversation_id, created_at);

        -- Index for listing conversations by update time
        CREATE INDEX IF NOT EXISTS idx_conversations_updated
        ON conversations(updated_at DESC);

        -- Schema version tracking
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );

        -- Insert initial version if not exists
        INSERT OR IGNORE INTO schema_version (version) VALUES (1);
        "#,
    )?;

    Ok(())
}

/// Get the current schema version
#[allow(dead_code)]
pub fn get_version(conn: &Connection) -> Result<i32> {
    let version: i32 = conn.query_row(
        "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tables() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"conversations".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"schema_version".to_string()));
    }

    #[test]
    fn test_schema_version() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        assert_eq!(get_version(&conn).unwrap(), SCHEMA_VERSION);
    }
}
