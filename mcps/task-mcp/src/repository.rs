//! Task repository for shared database access

use anyhow::{Context, Result};
use rusqlite::{params, types::FromSqlError, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::schema;
use crate::types::{Task, TaskDependency, TaskStatus};

/// New task input
#[derive(Debug, Clone)]
pub struct NewTask {
    pub title: String,
    pub description: String,
    pub priority: Option<i32>,
    pub plan_source: Option<String>,
    pub plan_section: Option<String>,
    pub assigned_to: Option<String>,
    pub parent_task_id: Option<String>,
    pub metadata: Option<String>,
}

/// Task filter for querying
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    pub status: Option<String>,
    pub plan_source: Option<String>,
    pub assigned_to: Option<String>,
    pub min_priority: Option<i32>,
    pub limit: Option<usize>,
}

/// Task repository with shared database access
#[derive(Clone)]
pub struct TaskRepository {
    db: Arc<Mutex<Connection>>,
}

impl TaskRepository {
    /// Create new repository and ensure tables exist
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {:?}", db_path))?;

        // Ensure schema exists
        schema::ensure_tables(&conn)?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create a new task
    pub fn create_task(&self, task: NewTask) -> Result<Task> {
        let conn = self.db.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();
        let priority = task.priority.unwrap_or(50);

        conn.execute(
            r#"
            INSERT INTO tasks (
                id, title, description, priority, plan_source, plan_section,
                created_at, assigned_to, parent_task_id, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                &id,
                &task.title,
                &task.description,
                priority,
                &task.plan_source,
                &task.plan_section,
                &created_at,
                &task.assigned_to,
                &task.parent_task_id,
                &task.metadata,
            ],
        )
        .context("Failed to create task")?;

        // Return the created task
        Ok(Task {
            id,
            title: task.title,
            description: task.description,
            status: TaskStatus::Pending,
            priority,
            plan_source: task.plan_source,
            plan_section: task.plan_section,
            created_at,
            started_at: None,
            completed_at: None,
            assigned_to: task.assigned_to,
            branch_name: None,
            pr_url: None,
            parent_task_id: task.parent_task_id,
            metadata: task.metadata,
        })
    }

    /// Get a task by ID or prefix (minimum 8 characters)
    pub fn get_task(&self, id: &str) -> Result<Option<Task>> {
        let conn = self.db.lock().unwrap();

        let task = conn
            .query_row(
                r#"
                SELECT id, title, description, status, priority, plan_source, plan_section,
                       created_at, started_at, completed_at, assigned_to, branch_name,
                       pr_url, parent_task_id, metadata
                FROM tasks
                WHERE id = ?1 OR id LIKE ?2
                "#,
                params![id, format!("{}%", id)],
                |row| {
                    let status_str: String = row.get(3)?;
                    let status = TaskStatus::from_str(&status_str)
                        .map_err(|e| FromSqlError::Other(format!("Invalid status: {}", e).into()))?;

                    Ok(Task {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        description: row.get(2)?,
                        status,
                        priority: row.get(4)?,
                        plan_source: row.get(5)?,
                        plan_section: row.get(6)?,
                        created_at: row.get(7)?,
                        started_at: row.get(8)?,
                        completed_at: row.get(9)?,
                        assigned_to: row.get(10)?,
                        branch_name: row.get(11)?,
                        pr_url: row.get(12)?,
                        parent_task_id: row.get(13)?,
                        metadata: row.get(14)?,
                    })
                },
            )
            .optional()
            .context("Failed to query task")?;

        Ok(task)
    }

    /// List tasks with optional filtering
    pub fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>> {
        let conn = self.db.lock().unwrap();

        let mut sql = String::from(
            r#"
            SELECT id, title, description, status, priority, plan_source, plan_section,
                   created_at, started_at, completed_at, assigned_to, branch_name,
                   pr_url, parent_task_id, metadata
            FROM tasks
            WHERE 1=1
            "#,
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(status) = &filter.status {
            sql.push_str(" AND status = ?");
            params.push(Box::new(status.clone()));
        }

        if let Some(plan_source) = &filter.plan_source {
            sql.push_str(" AND plan_source = ?");
            params.push(Box::new(plan_source.clone()));
        }

        if let Some(assigned_to) = &filter.assigned_to {
            sql.push_str(" AND assigned_to = ?");
            params.push(Box::new(assigned_to.clone()));
        }

        if let Some(min_priority) = filter.min_priority {
            sql.push_str(" AND priority >= ?");
            params.push(Box::new(min_priority));
        }

        sql.push_str(" ORDER BY priority DESC, created_at ASC");

        if let Some(limit) = filter.limit {
            sql.push_str(" LIMIT ?");
            params.push(Box::new(limit as i64));
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let tasks = stmt
            .query_map(param_refs.as_slice(), |row| {
                let status_str: String = row.get(3)?;
                let status = TaskStatus::from_str(&status_str)
                    .map_err(|e| FromSqlError::Other(format!("Invalid status: {}", e).into()))?;

                Ok(Task {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    status,
                    priority: row.get(4)?,
                    plan_source: row.get(5)?,
                    plan_section: row.get(6)?,
                    created_at: row.get(7)?,
                    started_at: row.get(8)?,
                    completed_at: row.get(9)?,
                    assigned_to: row.get(10)?,
                    branch_name: row.get(11)?,
                    pr_url: row.get(12)?,
                    parent_task_id: row.get(13)?,
                    metadata: row.get(14)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Update task status
    pub fn update_status(&self, id: &str, status: TaskStatus) -> Result<()> {
        let conn = self.db.lock().unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        let status_str = status.as_str();

        let mut sql = String::from("UPDATE tasks SET status = ?1");
        let mut params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(status_str.to_string()), Box::new(id.to_string())];

        match status {
            TaskStatus::InProgress => {
                sql.push_str(", started_at = ?3");
                params.insert(2, Box::new(now));
            }
            TaskStatus::Completed | TaskStatus::Failed => {
                sql.push_str(", completed_at = ?3");
                params.insert(2, Box::new(now));
            }
            _ => {}
        }

        sql.push_str(" WHERE id = ?2");

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .context("Failed to update task status")?;

        Ok(())
    }

    /// Update multiple task fields
    pub fn update_task_fields(
        &self,
        id: &str,
        status: Option<TaskStatus>,
        branch_name: Option<&str>,
        pr_url: Option<&str>,
        assigned_to: Option<&str>,
        priority: Option<i32>,
        metadata: Option<&str>,
    ) -> Result<()> {
        let conn = self.db.lock().unwrap();

        if let Some(status) = status {
            self.update_status_internal(&conn, id, status)?;
        }

        if let Some(branch) = branch_name {
            conn.execute(
                "UPDATE tasks SET branch_name = ?1 WHERE id = ?2",
                params![branch, id],
            )?;
        }

        if let Some(pr) = pr_url {
            conn.execute("UPDATE tasks SET pr_url = ?1 WHERE id = ?2", params![pr, id])?;
        }

        if let Some(assigned) = assigned_to {
            conn.execute(
                "UPDATE tasks SET assigned_to = ?1 WHERE id = ?2",
                params![assigned, id],
            )?;
        }

        if let Some(p) = priority {
            conn.execute(
                "UPDATE tasks SET priority = ?1 WHERE id = ?2",
                params![p, id],
            )?;
        }

        if let Some(meta) = metadata {
            conn.execute(
                "UPDATE tasks SET metadata = ?1 WHERE id = ?2",
                params![meta, id],
            )?;
        }

        Ok(())
    }

    /// Internal status update without locking (for use within other locked operations)
    fn update_status_internal(
        &self,
        conn: &Connection,
        id: &str,
        status: TaskStatus,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let status_str = status.as_str();

        let mut sql = String::from("UPDATE tasks SET status = ?1");
        let mut params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(status_str.to_string()), Box::new(id.to_string())];

        match status {
            TaskStatus::InProgress => {
                sql.push_str(", started_at = ?3");
                params.insert(2, Box::new(now));
            }
            TaskStatus::Completed | TaskStatus::Failed => {
                sql.push_str(", completed_at = ?3");
                params.insert(2, Box::new(now));
            }
            _ => {}
        }

        sql.push_str(" WHERE id = ?2");

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .context("Failed to update task status")?;

        Ok(())
    }

    /// Atomically grab the next pending task
    pub fn grab_next_task(
        &self,
        agent_name: &str,
        status_filter: Option<&str>,
    ) -> Result<Option<Task>> {
        let conn = self.db.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        // Start a transaction
        let tx = conn
            .unchecked_transaction()
            .context("Failed to start transaction")?;

        // Build the query to find the next pending task
        let status = status_filter.unwrap_or("pending");
        let sql = format!(
            r#"
            SELECT id FROM tasks
            WHERE status = '{}'
            AND id NOT IN (
                SELECT td.task_id
                FROM task_dependencies td
                JOIN tasks t ON t.id = td.depends_on_task_id
                WHERE t.status != 'completed'
            )
            ORDER BY priority DESC, created_at ASC
            LIMIT 1
            "#,
            status
        );

        let task_id: Option<String> = tx.query_row(&sql, [], |row| row.get(0)).optional()?;

        if let Some(task_id) = task_id {
            // Update the task to in_progress
            tx.execute(
                "UPDATE tasks SET status = 'in_progress', started_at = ?1, assigned_to = ?2 WHERE id = ?3",
                params![&now, agent_name, &task_id],
            )?;

            // Fetch the task within the transaction
            let task = tx.query_row(
                "SELECT id, title, description, status, priority, plan_source, plan_section, created_at, started_at, completed_at, assigned_to, branch_name, pr_url, parent_task_id, metadata FROM tasks WHERE id = ?1",
                params![&task_id],
                |row| {
                    let status_str: String = row.get(3)?;
                    let status = TaskStatus::from_str(&status_str)
                        .map_err(|e| FromSqlError::Other(format!("Invalid status: {}", e).into()))?;

                    Ok(Task {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        description: row.get(2)?,
                        status,
                        priority: row.get(4)?,
                        plan_source: row.get(5)?,
                        plan_section: row.get(6)?,
                        created_at: row.get(7)?,
                        started_at: row.get(8)?,
                        completed_at: row.get(9)?,
                        assigned_to: row.get(10)?,
                        branch_name: row.get(11)?,
                        pr_url: row.get(12)?,
                        parent_task_id: row.get(13)?,
                        metadata: row.get(14)?,
                    })
                },
            )?;

            tx.commit()?;
            Ok(Some(task))
        } else {
            tx.commit()?;
            Ok(None)
        }
    }

    /// Add a dependency between tasks
    pub fn add_dependency(&self, task_id: &str, depends_on: &str) -> Result<()> {
        let conn = self.db.lock().unwrap();
        let created_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO task_dependencies (task_id, depends_on_task_id, created_at) VALUES (?1, ?2, ?3)",
            params![task_id, depends_on, &created_at],
        )
        .context("Failed to add task dependency")?;

        Ok(())
    }

    /// Get all dependencies for a task
    pub fn get_dependencies(&self, task_id: &str) -> Result<Vec<TaskDependency>> {
        let conn = self.db.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT task_id, depends_on_task_id, created_at FROM task_dependencies WHERE task_id = ?1",
        )?;

        let deps = stmt
            .query_map(params![task_id], |row| {
                Ok(TaskDependency {
                    task_id: row.get(0)?,
                    depends_on_task_id: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(deps)
    }

    /// Check for incomplete dependencies (blocking tasks)
    pub fn check_blocking_tasks(&self, task_id: &str) -> Result<Vec<Task>> {
        let conn = self.db.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT t.id, t.title, t.description, t.status, t.priority, t.plan_source, t.plan_section,
                   t.created_at, t.started_at, t.completed_at, t.assigned_to, t.branch_name,
                   t.pr_url, t.parent_task_id, t.metadata
            FROM task_dependencies td
            JOIN tasks t ON t.id = td.depends_on_task_id
            WHERE td.task_id = ?1 AND t.status != 'completed'
            "#,
        )?;

        let tasks = stmt
            .query_map(params![task_id], |row| {
                let status_str: String = row.get(3)?;
                let status = TaskStatus::from_str(&status_str)
                    .map_err(|e| FromSqlError::Other(format!("Invalid status: {}", e).into()))?;

                Ok(Task {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    status,
                    priority: row.get(4)?,
                    plan_source: row.get(5)?,
                    plan_section: row.get(6)?,
                    created_at: row.get(7)?,
                    started_at: row.get(8)?,
                    completed_at: row.get(9)?,
                    assigned_to: row.get(10)?,
                    branch_name: row.get(11)?,
                    pr_url: row.get(12)?,
                    parent_task_id: row.get(13)?,
                    metadata: row.get(14)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Get tasks that are blocked by this task
    pub fn get_blocked_tasks(&self, task_id: &str) -> Result<Vec<Task>> {
        let conn = self.db.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT t.id, t.title, t.description, t.status, t.priority, t.plan_source, t.plan_section,
                   t.created_at, t.started_at, t.completed_at, t.assigned_to, t.branch_name,
                   t.pr_url, t.parent_task_id, t.metadata
            FROM task_dependencies td
            JOIN tasks t ON t.id = td.task_id
            WHERE td.depends_on_task_id = ?1
            "#,
        )?;

        let tasks = stmt
            .query_map(params![task_id], |row| {
                let status_str: String = row.get(3)?;
                let status = TaskStatus::from_str(&status_str)
                    .map_err(|e| FromSqlError::Other(format!("Invalid status: {}", e).into()))?;

                Ok(Task {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    status,
                    priority: row.get(4)?,
                    plan_source: row.get(5)?,
                    plan_section: row.get(6)?,
                    created_at: row.get(7)?,
                    started_at: row.get(8)?,
                    completed_at: row.get(9)?,
                    assigned_to: row.get(10)?,
                    branch_name: row.get(11)?,
                    pr_url: row.get(12)?,
                    parent_task_id: row.get(13)?,
                    metadata: row.get(14)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Record task execution
    pub fn record_execution(
        &self,
        task_id: &str,
        agent_name: &str,
        status: &str,
        run_id: Option<&str>,
        error: Option<&str>,
    ) -> Result<i64> {
        let conn = self.db.lock().unwrap();
        let started_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO task_executions (task_id, agent_name, status, run_id, error, started_at, completed_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
            "#,
            params![task_id, agent_name, status, run_id, error, &started_at],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Link a task to a run ID
    pub fn link_to_run(&self, task_id: &str, run_id: &str) -> Result<()> {
        let conn = self.db.lock().unwrap();

        // Update most recent execution with run_id
        conn.execute(
            r#"
            UPDATE task_executions
            SET run_id = ?1
            WHERE id = (
                SELECT id FROM task_executions
                WHERE task_id = ?2
                ORDER BY started_at DESC
                LIMIT 1
            )
            "#,
            params![run_id, task_id],
        )?;

        Ok(())
    }
}
