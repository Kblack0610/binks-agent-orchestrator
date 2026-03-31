//! Agent registry repository for shared database access

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use crate::schema;
use crate::types::*;

/// New agent input for registration
#[derive(Debug, Clone)]
pub struct NewAgent {
    pub name: String,
    pub agent_type: String,
    pub pid: Option<i64>,
    pub port: Option<i64>,
    pub working_directory: Option<String>,
    pub active_project: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub ttl_seconds: i64,
    pub metadata: Option<String>,
}

/// Filter for agent queries
#[derive(Debug, Clone, Default)]
pub struct AgentFilter {
    pub status: Option<String>,
    pub agent_type: Option<String>,
    pub active_project: Option<String>,
    pub include_stale: bool,
    pub limit: Option<usize>,
}

/// Filter for claim queries
#[derive(Debug, Clone, Default)]
pub struct ClaimFilter {
    pub agent_id: Option<String>,
    pub resource_type: Option<String>,
    pub active_only: bool,
    pub limit: Option<usize>,
}

/// Agent registry repository with shared database access
#[derive(Clone)]
pub struct AgentRegistryRepository {
    db: Arc<Mutex<Connection>>,
}

impl AgentRegistryRepository {
    /// Create new repository and ensure tables exist
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {:?}", db_path))?;

        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        schema::ensure_tables(&conn)?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
        })
    }

    // ========================================================================
    // Agent Lifecycle
    // ========================================================================

    /// Register a new agent, returns the created AgentRecord
    pub fn register_agent(&self, agent: NewAgent) -> Result<AgentRecord> {
        let conn = self.db.lock().unwrap();
        let agent_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let capabilities_json = agent
            .capabilities
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap_or_else(|_| "[]".to_string()));

        conn.execute(
            r#"
            INSERT INTO agent_registry (
                agent_id, name, agent_type, pid, port, working_directory,
                active_project, capabilities, status, ttl_seconds,
                last_heartbeat, registered_at, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', ?9, ?10, ?10, ?11)
            "#,
            params![
                &agent_id,
                &agent.name,
                &agent.agent_type,
                &agent.pid,
                &agent.port,
                &agent.working_directory,
                &agent.active_project,
                &capabilities_json,
                agent.ttl_seconds,
                &now,
                &agent.metadata,
            ],
        )
        .context("Failed to register agent")?;

        Ok(AgentRecord {
            agent_id,
            name: agent.name,
            agent_type: agent.agent_type,
            pid: agent.pid,
            port: agent.port,
            working_directory: agent.working_directory,
            active_project: agent.active_project,
            capabilities: agent.capabilities,
            status: AgentStatus::Active,
            ttl_seconds: agent.ttl_seconds,
            last_heartbeat: now.clone(),
            registered_at: now,
            deregistered_at: None,
            metadata: agent.metadata,
        })
    }

    /// Deregister an agent: mark as deregistered and release all claims
    pub fn deregister_agent(&self, agent_id: &str) -> Result<()> {
        let conn = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        let tx = conn
            .unchecked_transaction()
            .context("Failed to start transaction")?;

        // Mark agent as deregistered
        tx.execute(
            "UPDATE agent_registry SET status = 'deregistered', deregistered_at = ?1 WHERE agent_id = ?2",
            params![&now, agent_id],
        )?;

        // Release all port claims
        tx.execute(
            "UPDATE port_claims SET released_at = ?1 WHERE agent_id = ?2 AND released_at IS NULL",
            params![&now, agent_id],
        )?;

        // Release all resource claims
        tx.execute(
            "UPDATE resource_claims SET released_at = ?1 WHERE agent_id = ?2 AND released_at IS NULL",
            params![&now, agent_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Update heartbeat timestamp, optionally updating status/project/metadata
    pub fn heartbeat(
        &self,
        agent_id: &str,
        status: Option<&str>,
        active_project: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<AgentRecord> {
        let conn = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        let mut sql = String::from("UPDATE agent_registry SET last_heartbeat = ?1");
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now.clone())];

        if let Some(s) = status {
            // Validate status
            AgentStatus::from_str(s)
                .map_err(|e| anyhow::anyhow!("Invalid status: {}", e))?;
            sql.push_str(&format!(", status = ?{}", param_values.len() + 1));
            param_values.push(Box::new(s.to_string()));
        }

        if let Some(p) = active_project {
            sql.push_str(&format!(", active_project = ?{}", param_values.len() + 1));
            param_values.push(Box::new(p.to_string()));
        }

        if let Some(m) = metadata {
            sql.push_str(&format!(", metadata = ?{}", param_values.len() + 1));
            param_values.push(Box::new(m.to_string()));
        }

        sql.push_str(&format!(" WHERE agent_id = ?{}", param_values.len() + 1));
        param_values.push(Box::new(agent_id.to_string()));

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let updated = conn
            .execute(&sql, param_refs.as_slice())
            .context("Failed to update heartbeat")?;

        if updated == 0 {
            anyhow::bail!("Agent not found: {}", agent_id);
        }

        // Return the updated agent
        self.get_agent_internal(&conn, agent_id)?
            .ok_or_else(|| anyhow::anyhow!("Agent not found after heartbeat: {}", agent_id))
    }

    /// Update agent fields
    pub fn update_agent_fields(
        &self,
        agent_id: &str,
        status: Option<&str>,
        active_project: Option<&str>,
        working_directory: Option<&str>,
        capabilities: Option<&[String]>,
        metadata: Option<&str>,
    ) -> Result<()> {
        let conn = self.db.lock().unwrap();

        if let Some(s) = status {
            AgentStatus::from_str(s)
                .map_err(|e| anyhow::anyhow!("Invalid status: {}", e))?;
            conn.execute(
                "UPDATE agent_registry SET status = ?1 WHERE agent_id = ?2",
                params![s, agent_id],
            )?;
        }

        if let Some(p) = active_project {
            conn.execute(
                "UPDATE agent_registry SET active_project = ?1 WHERE agent_id = ?2",
                params![p, agent_id],
            )?;
        }

        if let Some(wd) = working_directory {
            conn.execute(
                "UPDATE agent_registry SET working_directory = ?1 WHERE agent_id = ?2",
                params![wd, agent_id],
            )?;
        }

        if let Some(caps) = capabilities {
            let json = serde_json::to_string(caps)?;
            conn.execute(
                "UPDATE agent_registry SET capabilities = ?1 WHERE agent_id = ?2",
                params![json, agent_id],
            )?;
        }

        if let Some(m) = metadata {
            conn.execute(
                "UPDATE agent_registry SET metadata = ?1 WHERE agent_id = ?2",
                params![m, agent_id],
            )?;
        }

        Ok(())
    }

    // ========================================================================
    // Agent Queries
    // ========================================================================

    /// Get agent by ID
    pub fn get_agent(&self, agent_id: &str) -> Result<Option<AgentRecord>> {
        let conn = self.db.lock().unwrap();
        self.get_agent_internal(&conn, agent_id)
    }

    /// Internal get_agent (doesn't lock — caller must hold the lock)
    fn get_agent_internal(
        &self,
        conn: &Connection,
        agent_id: &str,
    ) -> Result<Option<AgentRecord>> {
        let agent = conn
            .query_row(
                r#"
                SELECT agent_id, name, agent_type, pid, port, working_directory,
                       active_project, capabilities, status, ttl_seconds,
                       last_heartbeat, registered_at, deregistered_at, metadata
                FROM agent_registry
                WHERE agent_id = ?1
                "#,
                params![agent_id],
                Self::row_to_agent,
            )
            .optional()
            .context("Failed to query agent")?;

        Ok(agent)
    }

    /// List agents with optional filtering
    pub fn list_agents(&self, filter: AgentFilter) -> Result<Vec<AgentRecord>> {
        let conn = self.db.lock().unwrap();

        let mut sql = String::from(
            r#"
            SELECT agent_id, name, agent_type, pid, port, working_directory,
                   active_project, capabilities, status, ttl_seconds,
                   last_heartbeat, registered_at, deregistered_at, metadata
            FROM agent_registry
            WHERE 1=1
            "#,
        );

        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(status) = &filter.status {
            sql.push_str(&format!(" AND status = ?{}", param_values.len() + 1));
            param_values.push(Box::new(status.clone()));
        }

        if let Some(agent_type) = &filter.agent_type {
            sql.push_str(&format!(" AND agent_type = ?{}", param_values.len() + 1));
            param_values.push(Box::new(agent_type.clone()));
        }

        if let Some(project) = &filter.active_project {
            sql.push_str(&format!(
                " AND active_project LIKE ?{}",
                param_values.len() + 1
            ));
            param_values.push(Box::new(format!("%{}%", project)));
        }

        if !filter.include_stale {
            sql.push_str(" AND status NOT IN ('stale', 'deregistered')");
        }

        sql.push_str(" ORDER BY last_heartbeat DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT ?{}", param_values.len() + 1));
            param_values.push(Box::new(limit as i64));
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let agents = stmt
            .query_map(param_refs.as_slice(), Self::row_to_agent)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(agents)
    }

    // ========================================================================
    // Port Claims
    // ========================================================================

    /// Claim a port (first-come-first-served with conflict detection)
    pub fn claim_port(
        &self,
        agent_id: &str,
        port: i64,
        purpose: Option<&str>,
    ) -> Result<PortClaimResult> {
        let conn = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        let tx = conn
            .unchecked_transaction()
            .context("Failed to start transaction")?;

        // Check for active claim on this port (from a live agent)
        let existing: Option<(String, String, String)> = tx
            .query_row(
                r#"
                SELECT pc.agent_id, ar.name, pc.claimed_at
                FROM port_claims pc
                JOIN agent_registry ar ON ar.agent_id = pc.agent_id
                WHERE pc.port = ?1
                  AND pc.released_at IS NULL
                  AND ar.status NOT IN ('stale', 'deregistered')
                "#,
                params![port],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        if let Some((held_by_id, held_by_name, claimed_at)) = existing {
            if held_by_id == agent_id {
                // Idempotent: agent already holds this port
                tx.commit()?;
                return Ok(PortClaimResult {
                    success: true,
                    port,
                    agent_id: agent_id.to_string(),
                    conflict: None,
                });
            }
            // Conflict
            tx.commit()?;
            return Ok(PortClaimResult {
                success: false,
                port,
                agent_id: agent_id.to_string(),
                conflict: Some(PortConflict {
                    held_by_agent_id: held_by_id,
                    held_by_name,
                    claimed_at,
                }),
            });
        }

        // Release any stale claims on this port first
        tx.execute(
            "UPDATE port_claims SET released_at = ?1 WHERE port = ?2 AND released_at IS NULL",
            params![&now, port],
        )?;

        // Insert the new claim
        tx.execute(
            "INSERT INTO port_claims (port, agent_id, claimed_at, purpose) VALUES (?1, ?2, ?3, ?4)",
            params![port, agent_id, &now, purpose],
        )?;

        tx.commit()?;

        Ok(PortClaimResult {
            success: true,
            port,
            agent_id: agent_id.to_string(),
            conflict: None,
        })
    }

    /// Release a port claim
    pub fn release_port(&self, agent_id: &str, port: i64) -> Result<()> {
        let conn = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE port_claims SET released_at = ?1 WHERE agent_id = ?2 AND port = ?3 AND released_at IS NULL",
            params![&now, agent_id, port],
        )?;

        Ok(())
    }

    /// Query who holds a specific port
    pub fn who_has_port(&self, port: i64) -> Result<Option<(PortClaim, AgentRecord)>> {
        let conn = self.db.lock().unwrap();

        let result = conn
            .query_row(
                r#"
                SELECT pc.port, pc.agent_id, pc.claimed_at, pc.released_at, pc.purpose,
                       ar.agent_id, ar.name, ar.agent_type, ar.pid, ar.port, ar.working_directory,
                       ar.active_project, ar.capabilities, ar.status, ar.ttl_seconds,
                       ar.last_heartbeat, ar.registered_at, ar.deregistered_at, ar.metadata
                FROM port_claims pc
                JOIN agent_registry ar ON ar.agent_id = pc.agent_id
                WHERE pc.port = ?1
                  AND pc.released_at IS NULL
                  AND ar.status NOT IN ('stale', 'deregistered')
                "#,
                params![port],
                |row| {
                    let claim = PortClaim {
                        port: row.get(0)?,
                        agent_id: row.get(1)?,
                        claimed_at: row.get(2)?,
                        released_at: row.get(3)?,
                        purpose: row.get(4)?,
                    };

                    let capabilities_str: Option<String> = row.get(12)?;
                    let capabilities = capabilities_str
                        .as_deref()
                        .and_then(|s| serde_json::from_str(s).ok());

                    let status_str: String = row.get(13)?;
                    let status = AgentStatus::from_str(&status_str).unwrap_or(AgentStatus::Active);

                    let agent = AgentRecord {
                        agent_id: row.get(5)?,
                        name: row.get(6)?,
                        agent_type: row.get(7)?,
                        pid: row.get(8)?,
                        port: row.get(9)?,
                        working_directory: row.get(10)?,
                        active_project: row.get(11)?,
                        capabilities,
                        status,
                        ttl_seconds: row.get(14)?,
                        last_heartbeat: row.get(15)?,
                        registered_at: row.get(16)?,
                        deregistered_at: row.get(17)?,
                        metadata: row.get(18)?,
                    };

                    Ok((claim, agent))
                },
            )
            .optional()?;

        Ok(result)
    }

    // ========================================================================
    // Resource Claims
    // ========================================================================

    /// Claim a resource with exclusive/shared mode
    pub fn claim_resource(
        &self,
        agent_id: &str,
        resource_type: &str,
        resource_identifier: &str,
        exclusive: bool,
        purpose: Option<&str>,
    ) -> Result<ResourceClaimResult> {
        let conn = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let claim_id = uuid::Uuid::new_v4().to_string();

        let tx = conn
            .unchecked_transaction()
            .context("Failed to start transaction")?;

        // Check for conflicting active claims
        let conflict: Option<(String, String, String)> = if exclusive {
            // Exclusive claim conflicts with ANY active claim on the same resource
            tx.query_row(
                r#"
                SELECT rc.agent_id, ar.name, rc.claimed_at
                FROM resource_claims rc
                JOIN agent_registry ar ON ar.agent_id = rc.agent_id
                WHERE rc.resource_type = ?1
                  AND rc.resource_identifier = ?2
                  AND rc.released_at IS NULL
                  AND ar.status NOT IN ('stale', 'deregistered')
                  AND rc.agent_id != ?3
                "#,
                params![resource_type, resource_identifier, agent_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
        } else {
            // Shared claim only conflicts with EXCLUSIVE claims
            tx.query_row(
                r#"
                SELECT rc.agent_id, ar.name, rc.claimed_at
                FROM resource_claims rc
                JOIN agent_registry ar ON ar.agent_id = rc.agent_id
                WHERE rc.resource_type = ?1
                  AND rc.resource_identifier = ?2
                  AND rc.released_at IS NULL
                  AND rc.exclusive = 1
                  AND ar.status NOT IN ('stale', 'deregistered')
                  AND rc.agent_id != ?3
                "#,
                params![resource_type, resource_identifier, agent_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
        };

        if let Some((held_by_id, held_by_name, claimed_at)) = conflict {
            tx.commit()?;
            return Ok(ResourceClaimResult {
                success: false,
                claim_id: None,
                conflict: Some(ResourceConflict {
                    held_by_agent_id: held_by_id,
                    held_by_name,
                    resource_type: resource_type.to_string(),
                    resource_identifier: resource_identifier.to_string(),
                    claimed_at,
                }),
            });
        }

        // Insert the claim
        tx.execute(
            r#"
            INSERT INTO resource_claims (id, agent_id, resource_type, resource_identifier, exclusive, claimed_at, purpose)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                &claim_id,
                agent_id,
                resource_type,
                resource_identifier,
                exclusive as i32,
                &now,
                purpose,
            ],
        )?;

        tx.commit()?;

        Ok(ResourceClaimResult {
            success: true,
            claim_id: Some(claim_id),
            conflict: None,
        })
    }

    /// Release a specific resource claim
    pub fn release_resource(&self, claim_id: &str) -> Result<()> {
        let conn = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE resource_claims SET released_at = ?1 WHERE id = ?2 AND released_at IS NULL",
            params![&now, claim_id],
        )?;

        Ok(())
    }

    /// Release all claims (port + resource) for an agent
    pub fn release_all_for_agent(&self, agent_id: &str) -> Result<(usize, usize)> {
        let conn = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        let ports_released = conn.execute(
            "UPDATE port_claims SET released_at = ?1 WHERE agent_id = ?2 AND released_at IS NULL",
            params![&now, agent_id],
        )?;

        let resources_released = conn.execute(
            "UPDATE resource_claims SET released_at = ?1 WHERE agent_id = ?2 AND released_at IS NULL",
            params![&now, agent_id],
        )?;

        Ok((ports_released, resources_released))
    }

    // ========================================================================
    // Queries
    // ========================================================================

    /// Find which agents are working on a given resource
    pub fn who_is_working_on(
        &self,
        resource_identifier: &str,
        resource_type: Option<&str>,
    ) -> Result<WhoIsWorkingOnResponse> {
        let conn = self.db.lock().unwrap();

        // Find agents by active_project match
        let mut agents_by_project: Vec<AgentRecord> = {
            let mut stmt = conn.prepare(
                r#"
                SELECT agent_id, name, agent_type, pid, port, working_directory,
                       active_project, capabilities, status, ttl_seconds,
                       last_heartbeat, registered_at, deregistered_at, metadata
                FROM agent_registry
                WHERE active_project LIKE ?1
                  AND status NOT IN ('stale', 'deregistered')
                "#,
            )?;

            let result = stmt.query_map(params![format!("%{}%", resource_identifier)], |row| {
                Self::row_to_agent(row)
            })?
            .collect::<Result<Vec<_>, _>>()?;
            result
        };

        // Find active resource claims matching the identifier
        let mut claim_sql = String::from(
            r#"
            SELECT rc.id, rc.agent_id, rc.resource_type, rc.resource_identifier,
                   rc.exclusive, rc.claimed_at, rc.released_at, rc.purpose
            FROM resource_claims rc
            JOIN agent_registry ar ON ar.agent_id = rc.agent_id
            WHERE rc.resource_identifier LIKE ?1
              AND rc.released_at IS NULL
              AND ar.status NOT IN ('stale', 'deregistered')
            "#,
        );

        let mut claim_params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(format!("%{}%", resource_identifier))];

        if let Some(rt) = resource_type {
            claim_sql.push_str(&format!(" AND rc.resource_type = ?{}", claim_params.len() + 1));
            claim_params.push(Box::new(rt.to_string()));
        }

        let claim_param_refs: Vec<&dyn rusqlite::ToSql> =
            claim_params.iter().map(|p| p.as_ref()).collect();

        let claims: Vec<ResourceClaim> = {
            let mut stmt = conn.prepare(&claim_sql)?;
            let result = stmt.query_map(claim_param_refs.as_slice(), |row| {
                let exclusive_int: i32 = row.get(4)?;
                Ok(ResourceClaim {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    resource_type: row.get(2)?,
                    resource_identifier: row.get(3)?,
                    exclusive: exclusive_int != 0,
                    claimed_at: row.get(5)?,
                    released_at: row.get(6)?,
                    purpose: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
            result
        };

        // Also fetch agents from claims (dedup by agent_id)
        let existing_ids: std::collections::HashSet<String> =
            agents_by_project.iter().map(|a| a.agent_id.clone()).collect();

        for claim in &claims {
            if !existing_ids.contains(&claim.agent_id) {
                if let Some(agent) = self.get_agent_internal(&conn, &claim.agent_id)? {
                    agents_by_project.push(agent);
                }
            }
        }

        Ok(WhoIsWorkingOnResponse {
            resource_identifier: resource_identifier.to_string(),
            agents: agents_by_project,
            claims,
        })
    }

    /// List port and resource claims with filters
    pub fn list_claims(&self, filter: ClaimFilter) -> Result<(Vec<PortClaim>, Vec<ResourceClaim>)> {
        let conn = self.db.lock().unwrap();

        // Port claims
        let port_claims = {
            let mut sql = String::from(
                "SELECT port, agent_id, claimed_at, released_at, purpose FROM port_claims WHERE 1=1",
            );
            let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(aid) = &filter.agent_id {
                sql.push_str(&format!(" AND agent_id = ?{}", param_values.len() + 1));
                param_values.push(Box::new(aid.clone()));
            }

            if filter.active_only {
                sql.push_str(" AND released_at IS NULL");
            }

            sql.push_str(" ORDER BY claimed_at DESC");

            if let Some(limit) = filter.limit {
                sql.push_str(&format!(" LIMIT ?{}", param_values.len() + 1));
                param_values.push(Box::new(limit as i64));
            }

            let param_refs: Vec<&dyn rusqlite::ToSql> =
                param_values.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn.prepare(&sql)?;
            let result = stmt.query_map(param_refs.as_slice(), |row| {
                Ok(PortClaim {
                    port: row.get(0)?,
                    agent_id: row.get(1)?,
                    claimed_at: row.get(2)?,
                    released_at: row.get(3)?,
                    purpose: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
            result
        };

        // Resource claims
        let resource_claims = {
            let mut sql = String::from(
                r#"SELECT id, agent_id, resource_type, resource_identifier, exclusive,
                          claimed_at, released_at, purpose
                   FROM resource_claims WHERE 1=1"#,
            );
            let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(aid) = &filter.agent_id {
                sql.push_str(&format!(" AND agent_id = ?{}", param_values.len() + 1));
                param_values.push(Box::new(aid.clone()));
            }

            if let Some(rt) = &filter.resource_type {
                sql.push_str(&format!(" AND resource_type = ?{}", param_values.len() + 1));
                param_values.push(Box::new(rt.clone()));
            }

            if filter.active_only {
                sql.push_str(" AND released_at IS NULL");
            }

            sql.push_str(" ORDER BY claimed_at DESC");

            if let Some(limit) = filter.limit {
                sql.push_str(&format!(" LIMIT ?{}", param_values.len() + 1));
                param_values.push(Box::new(limit as i64));
            }

            let param_refs: Vec<&dyn rusqlite::ToSql> =
                param_values.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn.prepare(&sql)?;
            let result = stmt.query_map(param_refs.as_slice(), |row| {
                let exclusive_int: i32 = row.get(4)?;
                Ok(ResourceClaim {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    resource_type: row.get(2)?,
                    resource_identifier: row.get(3)?,
                    exclusive: exclusive_int != 0,
                    claimed_at: row.get(5)?,
                    released_at: row.get(6)?,
                    purpose: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
            result
        };

        Ok((port_claims, resource_claims))
    }

    // ========================================================================
    // Liveness / Cleanup
    // ========================================================================

    /// Clean up stale agents whose heartbeat has expired
    pub fn cleanup_stale(&self, dry_run: bool) -> Result<CleanupResult> {
        let conn = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        // Find stale agents (heartbeat + ttl < now)
        let stale_agents: Vec<AgentRecord> = {
            let mut stmt = conn.prepare(
                r#"
                SELECT agent_id, name, agent_type, pid, port, working_directory,
                       active_project, capabilities, status, ttl_seconds,
                       last_heartbeat, registered_at, deregistered_at, metadata
                FROM agent_registry
                WHERE status NOT IN ('stale', 'deregistered')
                  AND (julianday('now') - julianday(last_heartbeat)) * 86400 >= ttl_seconds
                "#,
            )?;

            let result = stmt.query_map([], Self::row_to_agent)?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };

        if dry_run || stale_agents.is_empty() {
            return Ok(CleanupResult {
                dry_run,
                expired_agents: stale_agents,
                released_port_claims: 0,
                released_resource_claims: 0,
            });
        }

        let tx = conn
            .unchecked_transaction()
            .context("Failed to start transaction")?;

        let mut total_port_claims = 0usize;
        let mut total_resource_claims = 0usize;

        for agent in &stale_agents {
            // Mark agent as stale
            tx.execute(
                "UPDATE agent_registry SET status = 'stale' WHERE agent_id = ?1",
                params![&agent.agent_id],
            )?;

            // Release port claims
            let ports = tx.execute(
                "UPDATE port_claims SET released_at = ?1 WHERE agent_id = ?2 AND released_at IS NULL",
                params![&now, &agent.agent_id],
            )?;
            total_port_claims += ports;

            // Release resource claims
            let resources = tx.execute(
                "UPDATE resource_claims SET released_at = ?1 WHERE agent_id = ?2 AND released_at IS NULL",
                params![&now, &agent.agent_id],
            )?;
            total_resource_claims += resources;
        }

        tx.commit()?;

        Ok(CleanupResult {
            dry_run,
            expired_agents: stale_agents,
            released_port_claims: total_port_claims,
            released_resource_claims: total_resource_claims,
        })
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    /// Convert a database row to an AgentRecord
    fn row_to_agent(row: &rusqlite::Row) -> rusqlite::Result<AgentRecord> {
        let capabilities_str: Option<String> = row.get(7)?;
        let capabilities = capabilities_str
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());

        let status_str: String = row.get(8)?;
        let status = AgentStatus::from_str(&status_str).unwrap_or(AgentStatus::Active);

        Ok(AgentRecord {
            agent_id: row.get(0)?,
            name: row.get(1)?,
            agent_type: row.get(2)?,
            pid: row.get(3)?,
            port: row.get(4)?,
            working_directory: row.get(5)?,
            active_project: row.get(6)?,
            capabilities,
            status,
            ttl_seconds: row.get(9)?,
            last_heartbeat: row.get(10)?,
            registered_at: row.get(11)?,
            deregistered_at: row.get(12)?,
            metadata: row.get(13)?,
        })
    }
}
