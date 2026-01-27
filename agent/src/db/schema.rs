//! Database schema definitions and migrations

use anyhow::Result;
use rusqlite::Connection;

/// Current schema version
pub const SCHEMA_VERSION: i32 = 2;

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

        -- ============================================================
        -- Run Tracking Tables (v2)
        -- ============================================================

        -- Workflow runs table
        -- One row per workflow execution
        CREATE TABLE IF NOT EXISTS runs (
            id TEXT PRIMARY KEY,
            workflow_name TEXT NOT NULL,
            task TEXT NOT NULL,
            status TEXT NOT NULL,  -- 'running', 'completed', 'failed', 'cancelled'
            model TEXT NOT NULL,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            duration_ms INTEGER,
            context TEXT,  -- JSON: final context map from workflow
            error TEXT,    -- Error message if failed
            metadata TEXT  -- JSON: additional metadata
        );

        -- Index for listing runs by time
        CREATE INDEX IF NOT EXISTS idx_runs_started
        ON runs(started_at DESC);

        -- Index for filtering by workflow
        CREATE INDEX IF NOT EXISTS idx_runs_workflow
        ON runs(workflow_name, started_at DESC);

        -- Index for filtering by status
        CREATE INDEX IF NOT EXISTS idx_runs_status
        ON runs(status, started_at DESC);

        -- Run events table
        -- Events emitted during a run (tool calls, iterations, etc.)
        CREATE TABLE IF NOT EXISTS run_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id TEXT NOT NULL,
            step_index INTEGER NOT NULL,  -- Which workflow step (0-indexed)
            event_type TEXT NOT NULL,     -- 'tool_start', 'tool_complete', 'iteration', etc.
            event_data TEXT NOT NULL,     -- JSON: full event payload
            timestamp TEXT NOT NULL,
            FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
        );

        -- Index for retrieving events by run
        CREATE INDEX IF NOT EXISTS idx_run_events_run
        ON run_events(run_id, step_index, timestamp);

        -- Run metrics table
        -- Aggregated metrics per run (computed after completion)
        CREATE TABLE IF NOT EXISTS run_metrics (
            run_id TEXT PRIMARY KEY,
            total_tool_calls INTEGER NOT NULL DEFAULT 0,
            successful_tool_calls INTEGER NOT NULL DEFAULT 0,
            failed_tool_calls INTEGER NOT NULL DEFAULT 0,
            total_iterations INTEGER NOT NULL DEFAULT 0,
            total_tokens_in INTEGER,      -- If available from model
            total_tokens_out INTEGER,     -- If available from model
            files_read INTEGER NOT NULL DEFAULT 0,
            files_modified INTEGER NOT NULL DEFAULT 0,
            tools_used TEXT,              -- JSON: {"tool_name": count, ...}
            step_durations TEXT,          -- JSON: [duration_ms, ...]
            FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
        );

        -- Improvements table
        -- Track insights and changes made based on run analysis
        CREATE TABLE IF NOT EXISTS improvements (
            id TEXT PRIMARY KEY,
            category TEXT NOT NULL,       -- 'prompt', 'workflow', 'agent', 'tool', 'other'
            description TEXT NOT NULL,
            related_runs TEXT,            -- JSON: ["run_id1", "run_id2", ...]
            changes_made TEXT,            -- Description of changes implemented
            impact TEXT,                  -- JSON: {"metric": "value", ...} measured improvement
            created_at TEXT NOT NULL,
            applied_at TEXT,              -- When the improvement was applied
            verified_at TEXT,             -- When impact was verified
            status TEXT NOT NULL DEFAULT 'proposed'  -- 'proposed', 'applied', 'verified', 'rejected'
        );

        -- Index for listing improvements by time
        CREATE INDEX IF NOT EXISTS idx_improvements_created
        ON improvements(created_at DESC);

        -- Index for filtering by category
        CREATE INDEX IF NOT EXISTS idx_improvements_category
        ON improvements(category, created_at DESC);

        -- Index for filtering by status
        CREATE INDEX IF NOT EXISTS idx_improvements_status
        ON improvements(status, created_at DESC);
        "#,
    )?;

    // Handle migrations
    migrate(conn)?;

    Ok(())
}

/// Run migrations to upgrade schema
fn migrate(conn: &Connection) -> Result<()> {
    // Check if schema_version table exists and has data
    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if current_version < 1 {
        // Fresh install - insert version 1
        conn.execute(
            "INSERT OR IGNORE INTO schema_version (version) VALUES (1)",
            [],
        )?;
    }

    if current_version < 2 {
        // Migration from v1 to v2
        // Tables are created with IF NOT EXISTS, so we just need to record the version
        conn.execute(
            "INSERT OR IGNORE INTO schema_version (version) VALUES (2)",
            [],
        )?;
        tracing::info!("Migrated database schema to version 2 (run tracking)");
    }

    Ok(())
}

/// Get the current schema version
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
        // New v2 tables
        assert!(tables.contains(&"runs".to_string()));
        assert!(tables.contains(&"run_events".to_string()));
        assert!(tables.contains(&"run_metrics".to_string()));
        assert!(tables.contains(&"improvements".to_string()));
    }

    #[test]
    fn test_schema_version() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        assert_eq!(get_version(&conn).unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn test_runs_table_columns() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();

        // Insert a test run
        conn.execute(
            r#"INSERT INTO runs (id, workflow_name, task, status, model, started_at)
               VALUES ('test-run', 'implement-feature', 'Add dark mode', 'completed', 'qwen3:14b', '2024-01-15T10:00:00Z')"#,
            [],
        )
        .unwrap();

        // Verify it was inserted
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM runs WHERE id = 'test-run'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_run_events_foreign_key() {
        let conn = Connection::open_in_memory().unwrap();
        // Enable foreign key enforcement
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        create_tables(&conn).unwrap();

        // Insert a run first
        conn.execute(
            r#"INSERT INTO runs (id, workflow_name, task, status, model, started_at)
               VALUES ('test-run', 'test', 'test task', 'running', 'test-model', '2024-01-15T10:00:00Z')"#,
            [],
        )
        .unwrap();

        // Insert an event
        conn.execute(
            r#"INSERT INTO run_events (run_id, step_index, event_type, event_data, timestamp)
               VALUES ('test-run', 0, 'tool_start', '{"name": "read_file"}', '2024-01-15T10:00:01Z')"#,
            [],
        )
        .unwrap();

        // Verify cascade delete
        conn.execute("DELETE FROM runs WHERE id = 'test-run'", [])
            .unwrap();
        let event_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM run_events WHERE run_id = 'test-run'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 0);
    }

    #[test]
    fn test_improvements_table() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();

        conn.execute(
            r#"INSERT INTO improvements (id, category, description, created_at, status)
               VALUES ('imp-1', 'prompt', 'Updated planner to check file existence', '2024-01-15T10:00:00Z', 'proposed')"#,
            [],
        )
        .unwrap();

        let status: String = conn
            .query_row(
                "SELECT status FROM improvements WHERE id = 'imp-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "proposed");
    }
}
