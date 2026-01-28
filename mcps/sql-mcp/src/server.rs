//! SQL MCP Server implementation

use crate::config::SqlConfig;
use mcp_common::{json_success, McpError};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use rusqlite::Connection;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// Parameter Types
// ============================================================================

/// Parameters for sql_query tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryParams {
    /// SQL query to execute. For read-only mode, only SELECT statements are allowed.
    pub query: String,
}

/// Parameters for sql_tables tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TablesParams {
    /// Optional pattern to filter table names (SQL LIKE pattern, e.g., 'user%')
    pub pattern: Option<String>,
}

/// Parameters for sql_schema tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SchemaParams {
    /// Name of the table to get schema for
    pub table: String,
}

/// Parameters for sql_explain tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExplainParams {
    /// SQL query to explain
    pub query: String,
}

// ============================================================================
// Response Types
// ============================================================================

/// Query result with column info and rows
#[derive(Debug, Serialize)]
pub struct QueryResult {
    /// Column names
    pub columns: Vec<String>,
    /// Rows as arrays of values
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Number of rows returned
    pub row_count: usize,
}

/// Table info
#[derive(Debug, Serialize)]
pub struct TableInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub table_type: String,
}

/// Column schema info
#[derive(Debug, Serialize)]
pub struct ColumnInfo {
    pub cid: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub notnull: bool,
    pub default_value: Option<String>,
    pub pk: bool,
}

/// Table schema result
#[derive(Debug, Serialize)]
pub struct SchemaResult {
    pub table: String,
    pub columns: Vec<ColumnInfo>,
    pub sql: Option<String>,
}

/// Explain query plan result
#[derive(Debug, Serialize)]
pub struct ExplainResult {
    pub query: String,
    pub plan: Vec<ExplainStep>,
}

/// Single step in query execution plan
#[derive(Debug, Serialize)]
pub struct ExplainStep {
    pub id: i64,
    pub parent: i64,
    pub detail: String,
}

// ============================================================================
// Server Implementation
// ============================================================================

/// SQL MCP Server
#[derive(Clone)]
pub struct SqlMcpServer {
    conn: Arc<Mutex<Connection>>,
    allow_writes: bool,
    tool_router: ToolRouter<Self>,
}

impl SqlMcpServer {
    /// Create a new SQL MCP server
    pub fn new() -> Self {
        let config = SqlConfig::load().unwrap_or_else(|e| {
            tracing::warn!("Failed to load config: {}. Using default.", e);
            SqlConfig::default()
        });

        let conn = Connection::open(&config.database.path).unwrap_or_else(|e| {
            tracing::error!("Failed to open database at {:?}: {}", config.database.path, e);
            // Create in-memory database as fallback
            Connection::open_in_memory().expect("Failed to create in-memory database")
        });

        // Set query timeout
        let _ = conn.busy_timeout(std::time::Duration::from_secs(config.database.timeout_secs));

        Self {
            conn: Arc::new(Mutex::new(conn)),
            allow_writes: config.database.allow_writes,
            tool_router: Self::tool_router(),
        }
    }

    /// Check if a query is a read-only SELECT statement
    fn is_read_only_query(query: &str) -> bool {
        let normalized = query.trim().to_uppercase();
        // Allow SELECT, EXPLAIN, PRAGMA (read operations)
        normalized.starts_with("SELECT")
            || normalized.starts_with("EXPLAIN")
            || normalized.starts_with("PRAGMA")
            || normalized.starts_with("WITH") // CTEs that end in SELECT
    }
}

impl Default for SqlMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl SqlMcpServer {
    /// Execute a SQL query and return results
    #[tool(description = "Execute a SQL query on the database. Returns column names and rows as JSON. In read-only mode (default), only SELECT, EXPLAIN, and PRAGMA statements are allowed.")]
    async fn sql_query(&self, Parameters(params): Parameters<QueryParams>) -> Result<CallToolResult, McpError> {
        // Check write permission
        if !self.allow_writes && !Self::is_read_only_query(&params.query) {
            return Err(McpError::internal_error(
                "Write operations are disabled. Set allow_writes=true in config to enable.",
                None,
            ));
        }

        let conn = self.conn.lock().await;

        // Prepare and execute the query
        let mut stmt = conn.prepare(&params.query).map_err(|e| {
            McpError::internal_error(format!("Failed to prepare query: {}", e), None)
        })?;

        let columns: Vec<String> = stmt
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let rows: Vec<Vec<serde_json::Value>> = stmt
            .query_map([], |row| {
                let mut values = Vec::with_capacity(columns.len());
                for i in 0..columns.len() {
                    let value: rusqlite::types::Value = row.get(i)?;
                    let json_value = match value {
                        rusqlite::types::Value::Null => serde_json::Value::Null,
                        rusqlite::types::Value::Integer(i) => serde_json::json!(i),
                        rusqlite::types::Value::Real(f) => serde_json::json!(f),
                        rusqlite::types::Value::Text(s) => serde_json::Value::String(s),
                        rusqlite::types::Value::Blob(b) => {
                            serde_json::Value::String(format!("<blob {} bytes>", b.len()))
                        }
                    };
                    values.push(json_value);
                }
                Ok(values)
            })
            .map_err(|e| McpError::internal_error(format!("Query failed: {}", e), None))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| McpError::internal_error(format!("Failed to read rows: {}", e), None))?;

        let result = QueryResult {
            row_count: rows.len(),
            columns,
            rows,
        };

        json_success(&result)
    }

    /// List tables in the database
    #[tool(description = "List all tables in the database. Optionally filter by name pattern using SQL LIKE syntax (e.g., 'user%' for tables starting with 'user').")]
    async fn sql_tables(&self, Parameters(params): Parameters<TablesParams>) -> Result<CallToolResult, McpError> {
        let conn = self.conn.lock().await;

        let query = match &params.pattern {
            Some(pattern) => format!(
                "SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') AND name LIKE '{}' ORDER BY name",
                pattern.replace('\'', "''") // Escape single quotes
            ),
            None => "SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') ORDER BY name".to_string(),
        };

        let mut stmt = conn.prepare(&query).map_err(|e| {
            McpError::internal_error(format!("Failed to query tables: {}", e), None)
        })?;

        let tables: Vec<TableInfo> = stmt
            .query_map([], |row| {
                Ok(TableInfo {
                    name: row.get(0)?,
                    table_type: row.get(1)?,
                })
            })
            .map_err(|e| McpError::internal_error(format!("Failed to list tables: {}", e), None))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| McpError::internal_error(format!("Failed to read table info: {}", e), None))?;

        json_success(&tables)
    }

    /// Get schema for a specific table
    #[tool(description = "Get the schema (column definitions) for a specific table. Returns column names, types, constraints, and the CREATE TABLE statement.")]
    async fn sql_schema(&self, Parameters(params): Parameters<SchemaParams>) -> Result<CallToolResult, McpError> {
        let conn = self.conn.lock().await;

        // Get column info using PRAGMA
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info('{}')", params.table.replace('\'', "''")))
            .map_err(|e| McpError::internal_error(format!("Failed to get schema: {}", e), None))?;

        let columns: Vec<ColumnInfo> = stmt
            .query_map([], |row| {
                Ok(ColumnInfo {
                    cid: row.get(0)?,
                    name: row.get(1)?,
                    data_type: row.get(2)?,
                    notnull: row.get::<_, i64>(3)? != 0,
                    default_value: row.get(4)?,
                    pk: row.get::<_, i64>(5)? != 0,
                })
            })
            .map_err(|e| McpError::internal_error(format!("Failed to query schema: {}", e), None))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| McpError::internal_error(format!("Failed to read column info: {}", e), None))?;

        if columns.is_empty() {
            return Err(McpError::internal_error(
                format!("Table '{}' not found", params.table),
                None,
            ));
        }

        // Get CREATE TABLE statement
        let sql: Option<String> = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name=?",
                [&params.table],
                |row| row.get(0),
            )
            .ok();

        let result = SchemaResult {
            table: params.table,
            columns,
            sql,
        };

        json_success(&result)
    }

    /// Explain query execution plan
    #[tool(description = "Get the execution plan for a SQL query. Useful for understanding query performance and optimization.")]
    async fn sql_explain(&self, Parameters(params): Parameters<ExplainParams>) -> Result<CallToolResult, McpError> {
        let conn = self.conn.lock().await;

        let explain_query = format!("EXPLAIN QUERY PLAN {}", params.query);

        let mut stmt = conn.prepare(&explain_query).map_err(|e| {
            McpError::internal_error(format!("Failed to explain query: {}", e), None)
        })?;

        let plan: Vec<ExplainStep> = stmt
            .query_map([], |row| {
                Ok(ExplainStep {
                    id: row.get(0)?,
                    parent: row.get(1)?,
                    detail: row.get(3)?,
                })
            })
            .map_err(|e| McpError::internal_error(format!("Failed to get plan: {}", e), None))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| McpError::internal_error(format!("Failed to read plan: {}", e), None))?;

        let result = ExplainResult {
            query: params.query,
            plan,
        };

        json_success(&result)
    }
}

#[tool_handler]
impl rmcp::ServerHandler for SqlMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mode = if self.allow_writes { "read-write" } else { "read-only" };
        ServerInfo {
            instructions: Some(format!(
                "SQL database query MCP server. Currently in {} mode. \
                Use sql_query to execute queries, sql_tables to list tables, \
                sql_schema to get table structure, and sql_explain to analyze query plans.",
                mode
            )),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
