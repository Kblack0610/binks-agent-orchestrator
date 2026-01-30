//! Self-Healing MCP server implementation

use anyhow::Context;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::handlers;

/// Self-Healing MCP server
pub struct SelfHealingMcpServer {
    pub db: Arc<Mutex<Connection>>,
    tool_router: ToolRouter<Self>,
}

impl SelfHealingMcpServer {
    /// Create a new Self-Healing MCP server
    pub fn new() -> Result<Self, anyhow::Error> {
        let db_path = Self::get_db_path()?;

        tracing::info!("Opening database at: {}", db_path.display());

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?;

        // Enable WAL mode for better concurrent access
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;",
        )?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            tool_router: Self::tool_router(),
        })
    }

    /// Get database path from environment or use default
    fn get_db_path() -> Result<PathBuf, anyhow::Error> {
        if let Ok(path) = std::env::var("DATABASE_PATH") {
            let expanded = shellexpand::tilde(&path);
            return Ok(PathBuf::from(expanded.as_ref()));
        }

        // Default to ~/.binks/conversations.db
        let home = dirs::home_dir().context("Failed to get home directory")?;
        Ok(home.join(".binks").join("conversations.db"))
    }
}

// Clone implementation for Arc<Mutex<Connection>>
impl Clone for SelfHealingMcpServer {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            tool_router: self.tool_router.clone(),
        }
    }
}

// MCP tool router
#[tool_router]
impl SelfHealingMcpServer {
    /// Analyze health of a specific run
    #[tool(
        description = "Analyze health of a specific run including success rate, duration, and issues"
    )]
    async fn analyze_run_health(
        &self,
        Parameters(params): Parameters<crate::params::AnalyzeRunHealthParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::analyze_run_health(self, params).await
    }

    /// Detect recurring failure patterns across runs
    #[tool(
        description = "Detect recurring failure patterns across runs based on error types and frequencies"
    )]
    async fn detect_failure_patterns(
        &self,
        Parameters(params): Parameters<crate::params::DetectPatternsParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::detect_failure_patterns(self, params).await
    }

    /// Compute success metrics per agent
    #[tool(
        description = "Compute success rates and performance metrics for each agent"
    )]
    async fn compute_agent_metrics(
        &self,
        Parameters(params): Parameters<crate::params::ComputeAgentMetricsParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::compute_agent_metrics(self, params).await
    }

    /// Compute tool reliability metrics
    #[tool(
        description = "Compute reliability metrics for MCP tools including success rates and common errors"
    )]
    async fn compute_tool_reliability(
        &self,
        Parameters(params): Parameters<crate::params::ComputeToolReliabilityParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::compute_tool_reliability(self, params).await
    }

    /// Propose an improvement based on a detected pattern
    #[tool(
        description = "Generate an improvement proposal based on a detected failure pattern"
    )]
    async fn propose_improvement(
        &self,
        Parameters(params): Parameters<crate::params::ProposeImprovementParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::propose_improvement(self, params).await
    }

    /// Test an improvement in simulation mode
    #[tool(
        description = "Test an improvement using simulation, canary, or sandbox mode"
    )]
    async fn test_improvement(
        &self,
        Parameters(params): Parameters<crate::params::TestImprovementParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::test_improvement(self, params).await
    }

    /// Apply an approved improvement
    #[tool(
        description = "Mark an improvement as applied and record the changes made"
    )]
    async fn apply_improvement(
        &self,
        Parameters(params): Parameters<crate::params::ApplyImprovementParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::apply_improvement(self, params).await
    }

    /// Verify the impact of an applied improvement
    #[tool(
        description = "Measure the actual impact of an applied improvement over a measurement window"
    )]
    async fn verify_improvement(
        &self,
        Parameters(params): Parameters<crate::params::VerifyImprovementParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::verify_improvement(self, params).await
    }

    /// Get overall system health dashboard
    #[tool(
        description = "Get a comprehensive dashboard of system health metrics and trends"
    )]
    async fn get_health_dashboard(
        &self,
        Parameters(params): Parameters<crate::params::GetHealthDashboardParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::get_health_dashboard(self, params).await
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for SelfHealingMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Self-Healing MCP server for workflow health analysis and automated improvement proposals. \
                 Analyzes run history from ~/.binks/conversations.db to detect patterns, propose fixes, and verify improvements. \
                 Integrates with inbox-mcp for notifications."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for SelfHealingMcpServer {
    fn default() -> Self {
        Self::new().expect("Failed to create SelfHealingMcpServer")
    }
}
