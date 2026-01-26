//! Persistent memory layer - SQLite-backed long-term storage

use chrono::Utc;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::types::{Entity, EntityWithFacts, Fact, Relation, Summary};

/// Persistent memory - SQLite-backed knowledge graph
#[derive(Clone)]
pub struct PersistentMemory {
    conn: Arc<Mutex<Connection>>,
}

impl PersistentMemory {
    /// Create a new persistent memory with the given database path
    pub fn new(db_path: PathBuf) -> SqliteResult<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;
        let memory = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        // Run migrations synchronously during initialization
        let rt = tokio::runtime::Handle::try_current();
        if rt.is_ok() {
            // We're in an async context, need to spawn blocking
            tokio::task::block_in_place(|| {
                let conn = memory.conn.blocking_lock();
                Self::init_schema_sync(&conn)
            })?;
        } else {
            // We're not in async context, just run directly
            let conn = memory.conn.blocking_lock();
            Self::init_schema_sync(&conn)?;
        }

        Ok(memory)
    }

    /// Initialize the database schema (synchronous version for startup)
    fn init_schema_sync(conn: &Connection) -> SqliteResult<()> {
        conn.execute_batch(
            r#"
            -- Entities table
            CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                entity_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name);
            CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);

            -- Facts table
            CREATE TABLE IF NOT EXISTS facts (
                id TEXT PRIMARY KEY,
                entity_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                source TEXT NOT NULL,
                confidence REAL NOT NULL,
                recorded_at TEXT NOT NULL,
                FOREIGN KEY (entity_id) REFERENCES entities(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_facts_entity ON facts(entity_id);
            CREATE INDEX IF NOT EXISTS idx_facts_key ON facts(key);

            -- Relations table
            CREATE TABLE IF NOT EXISTS relations (
                id TEXT PRIMARY KEY,
                from_entity_id TEXT NOT NULL,
                to_entity_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                recorded_at TEXT NOT NULL,
                FOREIGN KEY (from_entity_id) REFERENCES entities(id) ON DELETE CASCADE,
                FOREIGN KEY (to_entity_id) REFERENCES entities(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_relations_from ON relations(from_entity_id);
            CREATE INDEX IF NOT EXISTS idx_relations_to ON relations(to_entity_id);
            CREATE INDEX IF NOT EXISTS idx_relations_type ON relations(relation_type);

            -- Session summaries table
            CREATE TABLE IF NOT EXISTS summaries (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                content TEXT NOT NULL,
                thought_count INTEGER NOT NULL,
                session_start TEXT NOT NULL,
                session_end TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_summaries_session ON summaries(session_id);
            "#,
        )
    }

    // ========================================================================
    // Entity Operations
    // ========================================================================

    /// Create or get an entity by name
    pub async fn get_or_create_entity(
        &self,
        name: &str,
        entity_type: &str,
    ) -> SqliteResult<Entity> {
        let conn = self.conn.lock().await;
        let now = Utc::now().to_rfc3339();

        // Try to find existing
        let existing: Option<Entity> = conn
            .query_row(
                "SELECT id, name, entity_type, created_at, updated_at FROM entities WHERE name = ?1",
                params![name],
                |row| {
                    Ok(Entity {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        entity_type: row.get(2)?,
                        created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    })
                },
            )
            .ok();

        if let Some(entity) = existing {
            return Ok(entity);
        }

        // Create new entity
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO entities (id, name, entity_type, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, entity_type, now, now],
        )?;

        Ok(Entity {
            id,
            name: name.to_string(),
            entity_type: entity_type.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Get an entity by name
    #[allow(dead_code)]
    pub async fn get_entity(&self, name: &str) -> SqliteResult<Option<Entity>> {
        let conn = self.conn.lock().await;

        conn.query_row(
            "SELECT id, name, entity_type, created_at, updated_at FROM entities WHERE name = ?1",
            params![name],
            |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            },
        )
        .optional()
    }

    /// Delete an entity and all associated facts and relations
    pub async fn delete_entity(&self, name: &str) -> SqliteResult<(usize, usize)> {
        let conn = self.conn.lock().await;

        // Get entity ID first
        let entity_id: Option<String> = conn
            .query_row(
                "SELECT id FROM entities WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()?;

        let Some(entity_id) = entity_id else {
            return Ok((0, 0));
        };

        // Count facts and relations before deleting
        let facts_count: usize = conn.query_row(
            "SELECT COUNT(*) FROM facts WHERE entity_id = ?1",
            params![entity_id],
            |row| row.get(0),
        )?;

        let relations_count: usize = conn.query_row(
            "SELECT COUNT(*) FROM relations WHERE from_entity_id = ?1 OR to_entity_id = ?1",
            params![entity_id],
            |row| row.get(0),
        )?;

        // Delete the entity (cascade will handle facts and relations)
        conn.execute("DELETE FROM entities WHERE id = ?1", params![entity_id])?;

        Ok((facts_count, relations_count))
    }

    // ========================================================================
    // Fact Operations
    // ========================================================================

    /// Add a fact to an entity
    pub async fn add_fact(
        &self,
        entity_id: &str,
        key: &str,
        value: &str,
        source: &str,
        confidence: f32,
    ) -> SqliteResult<Fact> {
        let conn = self.conn.lock().await;
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO facts (id, entity_id, key, value, source, confidence, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, entity_id, key, value, source, confidence, now.to_rfc3339()],
        )?;

        // Update entity's updated_at
        conn.execute(
            "UPDATE entities SET updated_at = ?1 WHERE id = ?2",
            params![now.to_rfc3339(), entity_id],
        )?;

        Ok(Fact {
            id,
            entity_id: entity_id.to_string(),
            key: key.to_string(),
            value: value.to_string(),
            source: source.to_string(),
            confidence,
            recorded_at: now,
        })
    }

    /// Get all facts for an entity
    pub async fn get_facts(&self, entity_id: &str) -> SqliteResult<Vec<Fact>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, entity_id, key, value, source, confidence, recorded_at FROM facts WHERE entity_id = ?1 ORDER BY recorded_at DESC",
        )?;

        let facts = stmt
            .query_map(params![entity_id], |row| {
                Ok(Fact {
                    id: row.get(0)?,
                    entity_id: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    source: row.get(4)?,
                    confidence: row.get(5)?,
                    recorded_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(facts)
    }

    // ========================================================================
    // Relation Operations
    // ========================================================================

    /// Add a relation between two entities
    pub async fn add_relation(
        &self,
        from_entity_id: &str,
        to_entity_id: &str,
        relation_type: &str,
    ) -> SqliteResult<Relation> {
        let conn = self.conn.lock().await;
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO relations (id, from_entity_id, to_entity_id, relation_type, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, from_entity_id, to_entity_id, relation_type, now.to_rfc3339()],
        )?;

        Ok(Relation {
            id,
            from_entity_id: from_entity_id.to_string(),
            to_entity_id: to_entity_id.to_string(),
            relation_type: relation_type.to_string(),
            recorded_at: now,
        })
    }

    /// Get all relations for an entity (both directions)
    pub async fn get_relations(&self, entity_id: &str) -> SqliteResult<Vec<Relation>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, from_entity_id, to_entity_id, relation_type, recorded_at FROM relations WHERE from_entity_id = ?1 OR to_entity_id = ?1 ORDER BY recorded_at DESC",
        )?;

        let relations = stmt
            .query_map(params![entity_id], |row| {
                Ok(Relation {
                    id: row.get(0)?,
                    from_entity_id: row.get(1)?,
                    to_entity_id: row.get(2)?,
                    relation_type: row.get(3)?,
                    recorded_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(relations)
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Query entities by pattern (supports wildcards with %)
    pub async fn query_entities(&self, pattern: &str) -> SqliteResult<Vec<EntityWithFacts>> {
        // Collect entities in a separate scope to ensure stmt/conn are dropped before any await
        let entities: Vec<Entity> = {
            let conn = self.conn.lock().await;

            // Convert simple wildcards to SQL LIKE pattern
            let sql_pattern = pattern.replace('*', "%");

            let mut stmt = conn.prepare(
                "SELECT id, name, entity_type, created_at, updated_at FROM entities WHERE name LIKE ?1 ORDER BY updated_at DESC LIMIT 100",
            )?;

            let result = stmt
                .query_map(params![sql_pattern], |row| {
                    Ok(Entity {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        entity_type: row.get(2)?,
                        created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    })
                })?
                .collect::<SqliteResult<Vec<_>>>()?;
            result
            // stmt and conn dropped at end of this block
        };

        // Get facts for each entity (now safe to await)
        let mut results = Vec::new();
        for entity in entities {
            let facts = self.get_facts(&entity.id).await?;
            results.push(EntityWithFacts { entity, facts });
        }

        Ok(results)
    }

    /// Get all relations between entities matching a pattern
    #[allow(dead_code)]
    pub async fn query_relations(&self, entity_pattern: &str) -> SqliteResult<Vec<Relation>> {
        let conn = self.conn.lock().await;
        let sql_pattern = entity_pattern.replace('*', "%");

        let mut stmt = conn.prepare(
            r#"
            SELECT r.id, r.from_entity_id, r.to_entity_id, r.relation_type, r.recorded_at
            FROM relations r
            JOIN entities e1 ON r.from_entity_id = e1.id
            JOIN entities e2 ON r.to_entity_id = e2.id
            WHERE e1.name LIKE ?1 OR e2.name LIKE ?1
            ORDER BY r.recorded_at DESC
            LIMIT 100
            "#,
        )?;

        let relations = stmt
            .query_map(params![sql_pattern], |row| {
                Ok(Relation {
                    id: row.get(0)?,
                    from_entity_id: row.get(1)?,
                    to_entity_id: row.get(2)?,
                    relation_type: row.get(3)?,
                    recorded_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(relations)
    }

    // ========================================================================
    // Summary Operations
    // ========================================================================

    /// Save a session summary
    pub async fn save_summary(
        &self,
        session_id: &str,
        content: &str,
        thought_count: usize,
        session_start: chrono::DateTime<Utc>,
        session_end: chrono::DateTime<Utc>,
    ) -> SqliteResult<Summary> {
        let conn = self.conn.lock().await;
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO summaries (id, session_id, content, thought_count, session_start, session_end, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, session_id, content, thought_count, session_start.to_rfc3339(), session_end.to_rfc3339(), now.to_rfc3339()],
        )?;

        Ok(Summary {
            id,
            session_id: session_id.to_string(),
            content: content.to_string(),
            thought_count,
            session_start,
            session_end,
            created_at: now,
        })
    }

    /// Get recent summaries
    #[allow(dead_code)]
    pub async fn get_recent_summaries(&self, limit: usize) -> SqliteResult<Vec<Summary>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, content, thought_count, session_start, session_end, created_at FROM summaries ORDER BY created_at DESC LIMIT ?1",
        )?;

        let summaries = stmt
            .query_map(params![limit], |row| {
                Ok(Summary {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    content: row.get(2)?,
                    thought_count: row.get(3)?,
                    session_start: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    session_end: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(summaries)
    }
}

/// Extension trait to make Option handling easier
trait SqliteResultExt<T> {
    fn optional(self) -> SqliteResult<Option<T>>;
}

impl<T> SqliteResultExt<T> for SqliteResult<T> {
    fn optional(self) -> SqliteResult<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_entity_lifecycle() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let memory = PersistentMemory::new(db_path).unwrap();

        // Create entity
        let entity = memory
            .get_or_create_entity("test:project", "project")
            .await
            .unwrap();
        assert_eq!(entity.name, "test:project");
        assert_eq!(entity.entity_type, "project");

        // Get same entity again
        let entity2 = memory
            .get_or_create_entity("test:project", "project")
            .await
            .unwrap();
        assert_eq!(entity.id, entity2.id);

        // Add fact
        let fact = memory
            .add_fact(&entity.id, "language", "Rust", "user", 1.0)
            .await
            .unwrap();
        assert_eq!(fact.key, "language");

        // Query
        let results = memory.query_entities("test:*").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].facts.len(), 1);

        // Delete
        let (facts, relations) = memory.delete_entity("test:project").await.unwrap();
        assert_eq!(facts, 1);
        assert_eq!(relations, 0);

        // Verify deleted
        let entity = memory.get_entity("test:project").await.unwrap();
        assert!(entity.is_none());
    }
}
