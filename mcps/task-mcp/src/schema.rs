//! Database schema initialization for task-mcp
//!
//! This module ensures the required tables exist in the shared database.
//! The schema is defined in agent/src/db/schema.rs and is shared with the agent.

use anyhow::Result;
use rusqlite::Connection;

/// Ensure task-related tables exist
/// This is a minimal check - full schema is managed by the agent
pub fn ensure_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Tasks table
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            priority INTEGER DEFAULT 50,
            plan_source TEXT,
            plan_section TEXT,
            created_at TEXT NOT NULL,
            started_at TEXT,
            completed_at TEXT,
            assigned_to TEXT,
            branch_name TEXT,
            pr_url TEXT,
            parent_task_id TEXT,
            metadata TEXT,
            FOREIGN KEY (parent_task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );

        -- Task dependencies table
        CREATE TABLE IF NOT EXISTS task_dependencies (
            task_id TEXT NOT NULL,
            depends_on_task_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (task_id, depends_on_task_id),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (depends_on_task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );

        -- Task executions table
        CREATE TABLE IF NOT EXISTS task_executions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id TEXT NOT NULL,
            run_id TEXT,
            agent_name TEXT NOT NULL,
            status TEXT NOT NULL,
            error TEXT,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            duration_ms INTEGER,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );

        -- Indexes
        CREATE INDEX IF NOT EXISTS idx_tasks_status
        ON tasks(status, priority DESC, created_at);

        CREATE INDEX IF NOT EXISTS idx_tasks_plan
        ON tasks(plan_source, created_at);

        CREATE INDEX IF NOT EXISTS idx_task_executions_task
        ON task_executions(task_id, started_at DESC);
        "#,
    )?;

    Ok(())
}
