//! Database schema initialization for agent-registry-mcp
//!
//! Creates tables for agent presence, port claims, and resource claims
//! in the shared ~/.binks/conversations.db database.

use anyhow::Result;
use rusqlite::Connection;

/// Ensure agent registry tables exist
pub fn ensure_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Agent registry table
        CREATE TABLE IF NOT EXISTS agent_registry (
            agent_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            agent_type TEXT NOT NULL DEFAULT 'unknown',
            pid INTEGER,
            port INTEGER,
            working_directory TEXT,
            active_project TEXT,
            capabilities TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            ttl_seconds INTEGER NOT NULL DEFAULT 300,
            last_heartbeat TEXT NOT NULL,
            registered_at TEXT NOT NULL,
            deregistered_at TEXT,
            metadata TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_agent_registry_status
        ON agent_registry(status, last_heartbeat DESC);

        CREATE INDEX IF NOT EXISTS idx_agent_registry_project
        ON agent_registry(active_project, status);

        CREATE INDEX IF NOT EXISTS idx_agent_registry_name
        ON agent_registry(name, status);

        -- Port claims table
        CREATE TABLE IF NOT EXISTS port_claims (
            port INTEGER NOT NULL,
            agent_id TEXT NOT NULL,
            claimed_at TEXT NOT NULL,
            released_at TEXT,
            purpose TEXT,
            PRIMARY KEY (port, agent_id),
            FOREIGN KEY (agent_id) REFERENCES agent_registry(agent_id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_port_claims_active
        ON port_claims(port, released_at);

        -- Resource claims table
        CREATE TABLE IF NOT EXISTS resource_claims (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            resource_type TEXT NOT NULL,
            resource_identifier TEXT NOT NULL,
            exclusive INTEGER NOT NULL DEFAULT 1,
            claimed_at TEXT NOT NULL,
            released_at TEXT,
            purpose TEXT,
            FOREIGN KEY (agent_id) REFERENCES agent_registry(agent_id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_resource_claims_active
        ON resource_claims(resource_type, resource_identifier, released_at);

        CREATE INDEX IF NOT EXISTS idx_resource_claims_agent
        ON resource_claims(agent_id, released_at);
        "#,
    )?;

    Ok(())
}
