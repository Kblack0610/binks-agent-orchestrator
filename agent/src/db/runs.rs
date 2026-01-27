//! Run tracking and recording operations
//!
//! Provides persistence for workflow runs, events, metrics, and improvements.

use super::Database;
use crate::agent::events::{AgentEvent, EventReceiver};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Data Types
// ============================================================================

/// Run status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunStatus::Running => write!(f, "running"),
            RunStatus::Completed => write!(f, "completed"),
            RunStatus::Failed => write!(f, "failed"),
            RunStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for RunStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "running" => Ok(RunStatus::Running),
            "completed" => Ok(RunStatus::Completed),
            "failed" => Ok(RunStatus::Failed),
            "cancelled" => Ok(RunStatus::Cancelled),
            _ => Err(anyhow::anyhow!("Unknown run status: {}", s)),
        }
    }
}

/// A workflow run record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub workflow_name: String,
    pub task: String,
    pub status: RunStatus,
    pub model: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub context: Option<serde_json::Value>,
    pub error: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Summary view of a run (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub id: String,
    pub workflow_name: String,
    pub task: String,
    pub status: RunStatus,
    pub model: String,
    pub started_at: DateTime<Utc>,
    pub duration_ms: Option<i64>,
}

/// An event within a run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub id: i64,
    pub run_id: String,
    pub step_index: usize,
    pub event_type: String,
    pub event_data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

/// Aggregated metrics for a run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunMetrics {
    pub run_id: String,
    pub total_tool_calls: i32,
    pub successful_tool_calls: i32,
    pub failed_tool_calls: i32,
    pub total_iterations: i32,
    pub total_tokens_in: Option<i64>,
    pub total_tokens_out: Option<i64>,
    pub files_read: i32,
    pub files_modified: i32,
    pub tools_used: HashMap<String, i32>,
    pub step_durations: Vec<i64>,
}

/// Improvement category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImprovementCategory {
    Prompt,
    Workflow,
    Agent,
    Tool,
    Other,
}

impl std::fmt::Display for ImprovementCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImprovementCategory::Prompt => write!(f, "prompt"),
            ImprovementCategory::Workflow => write!(f, "workflow"),
            ImprovementCategory::Agent => write!(f, "agent"),
            ImprovementCategory::Tool => write!(f, "tool"),
            ImprovementCategory::Other => write!(f, "other"),
        }
    }
}

impl std::str::FromStr for ImprovementCategory {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "prompt" => Ok(ImprovementCategory::Prompt),
            "workflow" => Ok(ImprovementCategory::Workflow),
            "agent" => Ok(ImprovementCategory::Agent),
            "tool" => Ok(ImprovementCategory::Tool),
            "other" => Ok(ImprovementCategory::Other),
            _ => Err(anyhow::anyhow!("Unknown improvement category: {}", s)),
        }
    }
}

/// Improvement status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImprovementStatus {
    Proposed,
    Applied,
    Verified,
    Rejected,
}

impl std::fmt::Display for ImprovementStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImprovementStatus::Proposed => write!(f, "proposed"),
            ImprovementStatus::Applied => write!(f, "applied"),
            ImprovementStatus::Verified => write!(f, "verified"),
            ImprovementStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl std::str::FromStr for ImprovementStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "proposed" => Ok(ImprovementStatus::Proposed),
            "applied" => Ok(ImprovementStatus::Applied),
            "verified" => Ok(ImprovementStatus::Verified),
            "rejected" => Ok(ImprovementStatus::Rejected),
            _ => Err(anyhow::anyhow!("Unknown improvement status: {}", s)),
        }
    }
}

/// An improvement record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub id: String,
    pub category: ImprovementCategory,
    pub description: String,
    pub related_runs: Vec<String>,
    pub changes_made: Option<String>,
    pub impact: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub applied_at: Option<DateTime<Utc>>,
    pub verified_at: Option<DateTime<Utc>>,
    pub status: ImprovementStatus,
}

// ============================================================================
// Filter Types
// ============================================================================

/// Filter for listing runs
#[derive(Debug, Clone, Default)]
pub struct RunFilter {
    pub workflow_name: Option<String>,
    pub status: Option<RunStatus>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Filter for listing improvements
#[derive(Debug, Clone, Default)]
pub struct ImprovementFilter {
    pub category: Option<ImprovementCategory>,
    pub status: Option<ImprovementStatus>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// ============================================================================
// Database Operations
// ============================================================================

impl Database {
    // ------------------------------------------------------------------------
    // Runs
    // ------------------------------------------------------------------------

    /// Start a new run
    pub fn start_run(&self, workflow_name: &str, task: &str, model: &str) -> Result<Run> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO runs (id, workflow_name, task, status, model, started_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            (
                &id,
                workflow_name,
                task,
                RunStatus::Running.to_string(),
                model,
                now.to_rfc3339(),
            ),
        )
        .context("Failed to start run")?;

        Ok(Run {
            id,
            workflow_name: workflow_name.to_string(),
            task: task.to_string(),
            status: RunStatus::Running,
            model: model.to_string(),
            started_at: now,
            completed_at: None,
            duration_ms: None,
            context: None,
            error: None,
            metadata: None,
        })
    }

    /// Complete a run successfully
    pub fn complete_run(&self, id: &str, context: Option<&serde_json::Value>) -> Result<()> {
        let now = Utc::now();
        let context_json = context.map(|c| serde_json::to_string(c).unwrap());

        let conn = self.conn.lock().unwrap();

        // Get start time to calculate duration
        let started_at: String =
            conn.query_row("SELECT started_at FROM runs WHERE id = ?1", [id], |row| {
                row.get(0)
            })?;
        let started = DateTime::parse_from_rfc3339(&started_at)?.with_timezone(&Utc);
        let duration_ms = (now - started).num_milliseconds();

        conn.execute(
            r#"
            UPDATE runs
            SET status = ?1, completed_at = ?2, duration_ms = ?3, context = ?4
            WHERE id = ?5
            "#,
            (
                RunStatus::Completed.to_string(),
                now.to_rfc3339(),
                duration_ms,
                context_json,
                id,
            ),
        )?;

        Ok(())
    }

    /// Fail a run with an error
    pub fn fail_run(&self, id: &str, error: &str) -> Result<()> {
        let now = Utc::now();

        let conn = self.conn.lock().unwrap();

        // Get start time to calculate duration
        let started_at: String =
            conn.query_row("SELECT started_at FROM runs WHERE id = ?1", [id], |row| {
                row.get(0)
            })?;
        let started = DateTime::parse_from_rfc3339(&started_at)?.with_timezone(&Utc);
        let duration_ms = (now - started).num_milliseconds();

        conn.execute(
            r#"
            UPDATE runs
            SET status = ?1, completed_at = ?2, duration_ms = ?3, error = ?4
            WHERE id = ?5
            "#,
            (
                RunStatus::Failed.to_string(),
                now.to_rfc3339(),
                duration_ms,
                error,
                id,
            ),
        )?;

        Ok(())
    }

    /// Cancel a run
    pub fn cancel_run(&self, id: &str) -> Result<()> {
        let now = Utc::now();

        let conn = self.conn.lock().unwrap();

        // Get start time to calculate duration
        let started_at: String =
            conn.query_row("SELECT started_at FROM runs WHERE id = ?1", [id], |row| {
                row.get(0)
            })?;
        let started = DateTime::parse_from_rfc3339(&started_at)?.with_timezone(&Utc);
        let duration_ms = (now - started).num_milliseconds();

        conn.execute(
            r#"
            UPDATE runs
            SET status = ?1, completed_at = ?2, duration_ms = ?3
            WHERE id = ?4
            "#,
            (
                RunStatus::Cancelled.to_string(),
                now.to_rfc3339(),
                duration_ms,
                id,
            ),
        )?;

        Ok(())
    }

    /// Get a run by ID
    pub fn get_run(&self, id: &str) -> Result<Option<Run>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, workflow_name, task, status, model, started_at,
                   completed_at, duration_ms, context, error, metadata
            FROM runs
            WHERE id = ?1
            "#,
        )?;

        let result = stmt.query_row([id], |row| {
            let started_at: String = row.get(5)?;
            let completed_at: Option<String> = row.get(6)?;
            let context_str: Option<String> = row.get(8)?;
            let metadata_str: Option<String> = row.get(10)?;
            let status_str: String = row.get(3)?;

            Ok(Run {
                id: row.get(0)?,
                workflow_name: row.get(1)?,
                task: row.get(2)?,
                status: status_str.parse().unwrap_or(RunStatus::Running),
                model: row.get(4)?,
                started_at: DateTime::parse_from_rfc3339(&started_at)
                    .unwrap()
                    .with_timezone(&Utc),
                completed_at: completed_at.map(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .unwrap()
                        .with_timezone(&Utc)
                }),
                duration_ms: row.get(7)?,
                context: context_str.and_then(|s| serde_json::from_str(&s).ok()),
                error: row.get(9)?,
                metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
            })
        });

        match result {
            Ok(run) => Ok(Some(run)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List runs with optional filters
    pub fn list_runs(&self, filter: &RunFilter) -> Result<Vec<RunSummary>> {
        let conn = self.conn.lock().unwrap();
        let limit = filter.limit.unwrap_or(50);
        let offset = filter.offset.unwrap_or(0);

        let mut sql = String::from(
            r#"
            SELECT id, workflow_name, task, status, model, started_at, duration_ms
            FROM runs
            WHERE 1=1
            "#,
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(ref workflow) = filter.workflow_name {
            sql.push_str(" AND workflow_name = ?");
            params.push(Box::new(workflow.clone()));
        }
        if let Some(status) = filter.status {
            sql.push_str(" AND status = ?");
            params.push(Box::new(status.to_string()));
        }

        sql.push_str(" ORDER BY started_at DESC LIMIT ? OFFSET ?");
        params.push(Box::new(limit));
        params.push(Box::new(offset));

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let runs = stmt
            .query_map(params_refs.as_slice(), |row| {
                let started_at: String = row.get(5)?;
                let status_str: String = row.get(3)?;

                Ok(RunSummary {
                    id: row.get(0)?,
                    workflow_name: row.get(1)?,
                    task: row.get(2)?,
                    status: status_str.parse().unwrap_or(RunStatus::Running),
                    model: row.get(4)?,
                    started_at: DateTime::parse_from_rfc3339(&started_at)
                        .unwrap()
                        .with_timezone(&Utc),
                    duration_ms: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(runs)
    }

    // ------------------------------------------------------------------------
    // Run Events
    // ------------------------------------------------------------------------

    /// Record an event for a run
    pub fn record_event(&self, run_id: &str, step_index: usize, event: &AgentEvent) -> Result<()> {
        let now = Utc::now();
        let event_type = match event {
            AgentEvent::ProcessingStart { .. } => "processing_start",
            AgentEvent::ToolStart { .. } => "tool_start",
            AgentEvent::ToolComplete { .. } => "tool_complete",
            AgentEvent::Token { .. } => "token",
            AgentEvent::Thinking { .. } => "thinking",
            AgentEvent::Iteration { .. } => "iteration",
            AgentEvent::ResponseComplete { .. } => "response_complete",
            AgentEvent::Error { .. } => "error",
            AgentEvent::StepStarted { .. } => "step_started",
            AgentEvent::StepCompleted { .. } => "step_completed",
        };
        let event_data = serde_json::to_string(event)?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO run_events (run_id, step_index, event_type, event_data, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            (
                run_id,
                step_index as i32,
                event_type,
                event_data,
                now.to_rfc3339(),
            ),
        )?;

        Ok(())
    }

    /// Get all events for a run
    pub fn get_run_events(&self, run_id: &str) -> Result<Vec<RunEvent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, run_id, step_index, event_type, event_data, timestamp
            FROM run_events
            WHERE run_id = ?1
            ORDER BY timestamp ASC
            "#,
        )?;

        let events = stmt
            .query_map([run_id], |row| {
                let timestamp: String = row.get(5)?;
                let event_data_str: String = row.get(4)?;

                Ok(RunEvent {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    step_index: row.get::<_, i32>(2)? as usize,
                    event_type: row.get(3)?,
                    event_data: serde_json::from_str(&event_data_str)
                        .unwrap_or(serde_json::Value::Null),
                    timestamp: DateTime::parse_from_rfc3339(&timestamp)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    // ------------------------------------------------------------------------
    // Run Metrics
    // ------------------------------------------------------------------------

    /// Save metrics for a run
    pub fn save_run_metrics(&self, metrics: &RunMetrics) -> Result<()> {
        let tools_json = serde_json::to_string(&metrics.tools_used)?;
        let durations_json = serde_json::to_string(&metrics.step_durations)?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT OR REPLACE INTO run_metrics
            (run_id, total_tool_calls, successful_tool_calls, failed_tool_calls,
             total_iterations, total_tokens_in, total_tokens_out, files_read,
             files_modified, tools_used, step_durations)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            (
                &metrics.run_id,
                metrics.total_tool_calls,
                metrics.successful_tool_calls,
                metrics.failed_tool_calls,
                metrics.total_iterations,
                metrics.total_tokens_in,
                metrics.total_tokens_out,
                metrics.files_read,
                metrics.files_modified,
                tools_json,
                durations_json,
            ),
        )?;

        Ok(())
    }

    /// Get metrics for a run
    pub fn get_run_metrics(&self, run_id: &str) -> Result<Option<RunMetrics>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT run_id, total_tool_calls, successful_tool_calls, failed_tool_calls,
                   total_iterations, total_tokens_in, total_tokens_out, files_read,
                   files_modified, tools_used, step_durations
            FROM run_metrics
            WHERE run_id = ?1
            "#,
        )?;

        let result = stmt.query_row([run_id], |row| {
            let tools_str: Option<String> = row.get(9)?;
            let durations_str: Option<String> = row.get(10)?;

            Ok(RunMetrics {
                run_id: row.get(0)?,
                total_tool_calls: row.get(1)?,
                successful_tool_calls: row.get(2)?,
                failed_tool_calls: row.get(3)?,
                total_iterations: row.get(4)?,
                total_tokens_in: row.get(5)?,
                total_tokens_out: row.get(6)?,
                files_read: row.get(7)?,
                files_modified: row.get(8)?,
                tools_used: tools_str
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
                step_durations: durations_str
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
            })
        });

        match result {
            Ok(m) => Ok(Some(m)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ------------------------------------------------------------------------
    // Improvements
    // ------------------------------------------------------------------------

    /// Create a new improvement
    pub fn create_improvement(
        &self,
        category: ImprovementCategory,
        description: &str,
        related_runs: &[String],
    ) -> Result<Improvement> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let runs_json = serde_json::to_string(related_runs)?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO improvements (id, category, description, related_runs, created_at, status)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            (
                &id,
                category.to_string(),
                description,
                runs_json,
                now.to_rfc3339(),
                ImprovementStatus::Proposed.to_string(),
            ),
        )?;

        Ok(Improvement {
            id,
            category,
            description: description.to_string(),
            related_runs: related_runs.to_vec(),
            changes_made: None,
            impact: None,
            created_at: now,
            applied_at: None,
            verified_at: None,
            status: ImprovementStatus::Proposed,
        })
    }

    /// Get an improvement by ID
    pub fn get_improvement(&self, id: &str) -> Result<Option<Improvement>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, category, description, related_runs, changes_made, impact,
                   created_at, applied_at, verified_at, status
            FROM improvements
            WHERE id = ?1
            "#,
        )?;

        let result = stmt.query_row([id], |row| {
            let category_str: String = row.get(1)?;
            let runs_str: Option<String> = row.get(3)?;
            let impact_str: Option<String> = row.get(5)?;
            let created_at: String = row.get(6)?;
            let applied_at: Option<String> = row.get(7)?;
            let verified_at: Option<String> = row.get(8)?;
            let status_str: String = row.get(9)?;

            Ok(Improvement {
                id: row.get(0)?,
                category: category_str.parse().unwrap_or(ImprovementCategory::Other),
                description: row.get(2)?,
                related_runs: runs_str
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
                changes_made: row.get(4)?,
                impact: impact_str.and_then(|s| serde_json::from_str(&s).ok()),
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .unwrap()
                    .with_timezone(&Utc),
                applied_at: applied_at.map(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .unwrap()
                        .with_timezone(&Utc)
                }),
                verified_at: verified_at.map(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .unwrap()
                        .with_timezone(&Utc)
                }),
                status: status_str.parse().unwrap_or(ImprovementStatus::Proposed),
            })
        });

        match result {
            Ok(imp) => Ok(Some(imp)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List improvements with optional filters
    pub fn list_improvements(&self, filter: &ImprovementFilter) -> Result<Vec<Improvement>> {
        let conn = self.conn.lock().unwrap();
        let limit = filter.limit.unwrap_or(50);
        let offset = filter.offset.unwrap_or(0);

        let mut sql = String::from(
            r#"
            SELECT id, category, description, related_runs, changes_made, impact,
                   created_at, applied_at, verified_at, status
            FROM improvements
            WHERE 1=1
            "#,
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(category) = filter.category {
            sql.push_str(" AND category = ?");
            params.push(Box::new(category.to_string()));
        }
        if let Some(status) = filter.status {
            sql.push_str(" AND status = ?");
            params.push(Box::new(status.to_string()));
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");
        params.push(Box::new(limit));
        params.push(Box::new(offset));

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let improvements = stmt
            .query_map(params_refs.as_slice(), |row| {
                let category_str: String = row.get(1)?;
                let runs_str: Option<String> = row.get(3)?;
                let impact_str: Option<String> = row.get(5)?;
                let created_at: String = row.get(6)?;
                let applied_at: Option<String> = row.get(7)?;
                let verified_at: Option<String> = row.get(8)?;
                let status_str: String = row.get(9)?;

                Ok(Improvement {
                    id: row.get(0)?,
                    category: category_str.parse().unwrap_or(ImprovementCategory::Other),
                    description: row.get(2)?,
                    related_runs: runs_str
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default(),
                    changes_made: row.get(4)?,
                    impact: impact_str.and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .unwrap()
                        .with_timezone(&Utc),
                    applied_at: applied_at.map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc)
                    }),
                    verified_at: verified_at.map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc)
                    }),
                    status: status_str.parse().unwrap_or(ImprovementStatus::Proposed),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(improvements)
    }

    /// Apply an improvement (record changes made)
    pub fn apply_improvement(&self, id: &str, changes_made: &str) -> Result<()> {
        let now = Utc::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            UPDATE improvements
            SET status = ?1, changes_made = ?2, applied_at = ?3
            WHERE id = ?4
            "#,
            (
                ImprovementStatus::Applied.to_string(),
                changes_made,
                now.to_rfc3339(),
                id,
            ),
        )?;
        Ok(())
    }

    /// Verify an improvement with measured impact
    pub fn verify_improvement(&self, id: &str, impact: &serde_json::Value) -> Result<()> {
        let now = Utc::now();
        let impact_json = serde_json::to_string(impact)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            UPDATE improvements
            SET status = ?1, impact = ?2, verified_at = ?3
            WHERE id = ?4
            "#,
            (
                ImprovementStatus::Verified.to_string(),
                impact_json,
                now.to_rfc3339(),
                id,
            ),
        )?;
        Ok(())
    }
}

// ============================================================================
// RunRecorder - Records events from an agent during workflow execution
// ============================================================================

/// Records events from an agent's event stream to the database
pub struct RunRecorder {
    db: Database,
    run_id: String,
    current_step: Arc<AtomicUsize>,
    metrics: Arc<std::sync::Mutex<RunMetrics>>,
}

impl RunRecorder {
    /// Create a new recorder for a run
    pub fn new(db: Database, run_id: String) -> Self {
        let metrics = RunMetrics {
            run_id: run_id.clone(),
            ..Default::default()
        };

        Self {
            db,
            run_id,
            current_step: Arc::new(AtomicUsize::new(0)),
            metrics: Arc::new(std::sync::Mutex::new(metrics)),
        }
    }

    /// Set the current workflow step index
    pub fn set_step(&self, step_index: usize) {
        self.current_step.store(step_index, Ordering::SeqCst);
    }

    /// Get the run ID
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Record a single event
    pub fn record(&self, event: &AgentEvent) -> Result<()> {
        let step = self.current_step.load(Ordering::SeqCst);

        // Update metrics based on event type
        {
            let mut metrics = self.metrics.lock().unwrap();
            match event {
                AgentEvent::ToolStart { name, .. } => {
                    metrics.total_tool_calls += 1;
                    *metrics.tools_used.entry(name.clone()).or_insert(0) += 1;

                    // Track file operations
                    if name.contains("read_file") || name.contains("get_file") {
                        metrics.files_read += 1;
                    }
                }
                AgentEvent::ToolComplete { is_error, name, .. } => {
                    if *is_error {
                        metrics.failed_tool_calls += 1;
                    } else {
                        metrics.successful_tool_calls += 1;
                    }

                    // Track file modifications
                    if !*is_error
                        && (name.contains("write")
                            || name.contains("edit")
                            || name.contains("create"))
                    {
                        metrics.files_modified += 1;
                    }
                }
                AgentEvent::Iteration { .. } => {
                    metrics.total_iterations += 1;
                }
                AgentEvent::ResponseComplete { .. } => {
                    // Could track response stats here
                }
                AgentEvent::StepStarted { step_index, .. } => {
                    // Update current step index for subsequent events
                    self.current_step.store(*step_index, Ordering::SeqCst);
                }
                AgentEvent::StepCompleted { duration_ms, .. } => {
                    // Track step duration
                    metrics.step_durations.push(*duration_ms as i64);
                }
                _ => {}
            }
        }

        // Persist the event (skip tokens to reduce noise)
        if !matches!(event, AgentEvent::Token { .. }) {
            self.db.record_event(&self.run_id, step, event)?;
        }

        Ok(())
    }

    /// Consume events from a channel until it closes
    pub async fn consume_events(self, mut rx: EventReceiver) {
        while let Some(event) = rx.recv().await {
            if let Err(e) = self.record(&event) {
                tracing::warn!("Failed to record event: {}", e);
            }
        }

        // Save final metrics
        if let Err(e) = self.save_metrics() {
            tracing::warn!("Failed to save metrics: {}", e);
        }
    }

    /// Save the accumulated metrics
    pub fn save_metrics(&self) -> Result<()> {
        let metrics = self.metrics.lock().unwrap();
        self.db.save_run_metrics(&metrics)
    }

    /// Add a step duration
    pub fn add_step_duration(&self, duration_ms: i64) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.step_durations.push(duration_ms);
    }
}

// ============================================================================
// Tests
// ============================================================================

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
    fn test_start_and_get_run() {
        let (db, _dir) = test_db();
        let run = db
            .start_run("implement-feature", "Add dark mode", "qwen3:14b")
            .unwrap();

        assert_eq!(run.workflow_name, "implement-feature");
        assert_eq!(run.task, "Add dark mode");
        assert_eq!(run.status, RunStatus::Running);

        let fetched = db.get_run(&run.id).unwrap().unwrap();
        assert_eq!(fetched.id, run.id);
    }

    #[test]
    fn test_complete_run() {
        let (db, _dir) = test_db();
        let run = db.start_run("test", "test task", "model").unwrap();

        let context = serde_json::json!({"plan": "step 1, step 2"});
        db.complete_run(&run.id, Some(&context)).unwrap();

        let fetched = db.get_run(&run.id).unwrap().unwrap();
        assert_eq!(fetched.status, RunStatus::Completed);
        assert!(fetched.duration_ms.is_some());
        assert!(fetched.context.is_some());
    }

    #[test]
    fn test_fail_run() {
        let (db, _dir) = test_db();
        let run = db.start_run("test", "test task", "model").unwrap();

        db.fail_run(&run.id, "Something went wrong").unwrap();

        let fetched = db.get_run(&run.id).unwrap().unwrap();
        assert_eq!(fetched.status, RunStatus::Failed);
        assert_eq!(fetched.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_list_runs() {
        let (db, _dir) = test_db();

        db.start_run("workflow-a", "task 1", "model").unwrap();
        db.start_run("workflow-b", "task 2", "model").unwrap();
        db.start_run("workflow-a", "task 3", "model").unwrap();

        let all = db.list_runs(&RunFilter::default()).unwrap();
        assert_eq!(all.len(), 3);

        let filtered = db
            .list_runs(&RunFilter {
                workflow_name: Some("workflow-a".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_record_and_get_events() {
        let (db, _dir) = test_db();
        let run = db.start_run("test", "test", "model").unwrap();

        let event = AgentEvent::ToolStart {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "/tmp/test.txt"}),
        };
        db.record_event(&run.id, 0, &event).unwrap();

        let events = db.get_run_events(&run.id).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "tool_start");
    }

    #[test]
    fn test_run_metrics() {
        let (db, _dir) = test_db();
        let run = db.start_run("test", "test", "model").unwrap();

        let mut metrics = RunMetrics {
            run_id: run.id.clone(),
            total_tool_calls: 10,
            successful_tool_calls: 8,
            failed_tool_calls: 2,
            ..Default::default()
        };
        metrics.tools_used.insert("read_file".to_string(), 5);
        metrics.tools_used.insert("edit_file".to_string(), 5);

        db.save_run_metrics(&metrics).unwrap();

        let fetched = db.get_run_metrics(&run.id).unwrap().unwrap();
        assert_eq!(fetched.total_tool_calls, 10);
        assert_eq!(fetched.tools_used.get("read_file"), Some(&5));
    }

    #[test]
    fn test_improvements() {
        let (db, _dir) = test_db();
        let run = db.start_run("test", "test", "model").unwrap();

        let imp = db
            .create_improvement(
                ImprovementCategory::Prompt,
                "Updated planner to verify paths",
                std::slice::from_ref(&run.id),
            )
            .unwrap();

        assert_eq!(imp.status, ImprovementStatus::Proposed);

        db.apply_improvement(&imp.id, "Modified planner.md system prompt")
            .unwrap();

        let fetched = db.get_improvement(&imp.id).unwrap().unwrap();
        assert_eq!(fetched.status, ImprovementStatus::Applied);
        assert!(fetched.changes_made.is_some());
    }

    #[test]
    fn test_run_recorder() {
        let (db, _dir) = test_db();
        let run = db.start_run("test", "test", "model").unwrap();

        let recorder = RunRecorder::new(db.clone(), run.id.clone());

        // Record some events
        recorder
            .record(&AgentEvent::ToolStart {
                name: "read_file".to_string(),
                arguments: serde_json::json!({}),
            })
            .unwrap();

        recorder
            .record(&AgentEvent::ToolComplete {
                name: "read_file".to_string(),
                result: "content".to_string(),
                duration: std::time::Duration::from_millis(100),
                is_error: false,
            })
            .unwrap();

        recorder.save_metrics().unwrap();

        let metrics = db.get_run_metrics(&run.id).unwrap().unwrap();
        assert_eq!(metrics.total_tool_calls, 1);
        assert_eq!(metrics.successful_tool_calls, 1);
        assert_eq!(metrics.files_read, 1);
    }
}
