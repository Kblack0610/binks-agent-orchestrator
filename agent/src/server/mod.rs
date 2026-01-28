//! MCP Server implementation
//!
//! This module exposes the agent as an MCP server, allowing other tools
//! to interact with the LLM and agent capabilities.
//!
//! Tools exposed:
//! - `chat` - Send a message to the LLM (simple chat, no tools)
//! - `agent_chat` - Send a message through the tool-using agent
//! - `list_tools` - List available MCP tools from connected servers

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent::{detect_capabilities, Agent, DirectMessage, ModelCapabilityOverride};
use crate::config::AgentSectionConfig;
use crate::db::runs::{ImprovementFilter, Run, RunEvent, RunFilter, RunMetrics, RunStatus};
use crate::db::Database;
use crate::llm::{Llm, OllamaClient};
use crate::mcp::McpClientPool;

/// Configuration for the agent MCP server
#[derive(Clone)]
pub struct ServerConfig {
    pub ollama_url: String,
    pub model: String,
    pub system_prompt: Option<String>,
    /// Enable run tracking/analysis tools (requires database)
    pub enable_runs: bool,
    /// Agent stability settings from .agent.toml
    pub agent_config: AgentSectionConfig,
    /// Model capability overrides from config
    pub model_overrides: HashMap<String, ModelCapabilityOverride>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            model: "qwen3-coder:30b".to_string(),
            system_prompt: None,
            enable_runs: true,
            agent_config: AgentSectionConfig::default(),
            model_overrides: HashMap::new(),
        }
    }
}

/// The Agent MCP Server
///
/// Exposes LLM chat and agent capabilities as MCP tools
pub struct AgentMcpServer {
    tool_router: ToolRouter<Self>,
    config: ServerConfig,
    /// Lazy-initialized agent (needs MCP pool which is async)
    agent: Arc<Mutex<Option<Agent>>>,
    /// Simple LLM client for non-agent chat
    llm: OllamaClient,
    /// Session storage for conversation history
    sessions: Arc<Mutex<HashMap<String, Vec<DirectMessage>>>>,
    /// Database for run tracking (optional)
    db: Option<Database>,
}

// ============================================================================
// Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChatParams {
    #[schemars(description = "The message to send to the LLM")]
    pub message: String,
    #[schemars(description = "Optional system prompt to set context")]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AgentChatParams {
    #[schemars(description = "The message to send to the agent")]
    pub message: String,
    #[schemars(description = "Optional system prompt for the agent")]
    pub system_prompt: Option<String>,
    #[schemars(
        description = "Optional list of MCP server names to filter tools (e.g., ['sysinfo', 'kubernetes']). Recommended for smaller models that struggle with many tools."
    )]
    pub servers: Option<Vec<String>>,
    #[schemars(
        description = "Session ID for conversation continuity. Omit for stateless single-turn calls."
    )]
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ClearSessionParams {
    #[schemars(description = "The session ID to clear")]
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListToolsParams {
    #[schemars(description = "Optional server name to filter tools")]
    pub server: Option<String>,
}

// ============================================================================
// Run Analysis Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListRunsParams {
    #[schemars(description = "Filter by workflow name")]
    pub workflow: Option<String>,
    #[schemars(description = "Filter by status (running, completed, failed, cancelled)")]
    pub status: Option<String>,
    #[schemars(description = "Maximum number of runs to return (default: 20)")]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetRunParams {
    #[schemars(description = "Run ID or prefix (minimum 8 characters)")]
    pub id: String,
    #[schemars(description = "Include events in the response")]
    pub include_events: Option<bool>,
    #[schemars(description = "Include metrics in the response")]
    pub include_metrics: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExportRunParams {
    #[schemars(description = "Run ID or prefix (minimum 8 characters)")]
    pub id: String,
    #[schemars(description = "Export format: 'markdown' or 'json' (default: markdown)")]
    pub format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListImprovementsParams {
    #[schemars(description = "Filter by category (prompt, workflow, agent, tool, other)")]
    pub category: Option<String>,
    #[schemars(description = "Filter by status (proposed, applied, verified, rejected)")]
    pub status: Option<String>,
    #[schemars(description = "Maximum number of improvements to return (default: 20)")]
    pub limit: Option<u32>,
}

// ============================================================================
// Tool Router Implementation
// ============================================================================

#[tool_router]
impl AgentMcpServer {
    pub fn new(config: ServerConfig) -> Self {
        let llm = OllamaClient::new(&config.ollama_url, &config.model);

        // Initialize database for runs if enabled
        let db = if config.enable_runs {
            match Database::open() {
                Ok(db) => {
                    tracing::info!("Run tracking database initialized");
                    Some(db)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to open runs database: {} - run tools will be unavailable",
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        Self {
            tool_router: Self::tool_router(),
            config,
            agent: Arc::new(Mutex::new(None)),
            llm,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            db,
        }
    }

    /// Initialize the agent with MCP pool (call this before serving)
    pub async fn init_agent(&self) -> Result<(), anyhow::Error> {
        let pool = McpClientPool::load()?.ok_or_else(|| anyhow::anyhow!("No .mcp.json found"))?;

        // Detect model capabilities
        let capabilities = detect_capabilities(
            &self.config.ollama_url,
            &self.config.model,
            Some(&self.config.model_overrides),
        )
        .await;

        tracing::info!(
            "Model capabilities for {}: tool_calling={}, thinking={}, format={:?}",
            self.config.model,
            capabilities.tool_calling,
            capabilities.thinking,
            capabilities.function_call_format
        );

        let mut agent = Agent::from_agent_config(
            &self.config.ollama_url,
            &self.config.model,
            pool,
            &self.config.agent_config,
        )
        .with_capabilities(capabilities);

        if let Some(ref prompt) = self.config.system_prompt {
            agent = agent.with_system_prompt(prompt);
        }

        let mut guard = self.agent.lock().await;
        *guard = Some(agent);

        Ok(())
    }

    // ========================================================================
    // Chat Tools
    // ========================================================================

    #[tool(
        description = "Send a message to the LLM and get a response. This is simple chat without tool access."
    )]
    async fn chat(
        &self,
        Parameters(params): Parameters<ChatParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("chat: {}", params.message);

        // Build messages with optional system prompt
        let response = if let Some(system) = params.system_prompt {
            // For system prompt, we need to use a different approach
            // Simple chat doesn't support system prompts easily, so just prepend
            let full_message = format!("System: {}\n\nUser: {}", system, params.message);
            self.llm
                .chat(&full_message)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        } else {
            self.llm
                .chat(&params.message)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        };

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// Lazily initialize the agent on first use
    async fn ensure_agent(&self) -> Result<(), McpError> {
        let mut guard = self.agent.lock().await;
        if guard.is_none() {
            tracing::info!("Lazily initializing agent with MCP tools...");

            let pool = McpClientPool::load()
                .map_err(|e| {
                    McpError::internal_error(format!("Failed to load MCP config: {}", e), None)
                })?
                .ok_or_else(|| McpError::internal_error("No .mcp.json found".to_string(), None))?;

            // Detect model capabilities
            let capabilities = detect_capabilities(
                &self.config.ollama_url,
                &self.config.model,
                Some(&self.config.model_overrides),
            )
            .await;

            tracing::info!(
                "Model capabilities for {}: tool_calling={}, thinking={}, format={:?}",
                self.config.model,
                capabilities.tool_calling,
                capabilities.thinking,
                capabilities.function_call_format
            );

            let mut agent = Agent::from_agent_config(
                &self.config.ollama_url,
                &self.config.model,
                pool,
                &self.config.agent_config,
            )
            .with_capabilities(capabilities);

            if let Some(ref prompt) = self.config.system_prompt {
                agent = agent.with_system_prompt(prompt);
            }

            *guard = Some(agent);
            tracing::info!("Agent initialized successfully");
        }
        Ok(())
    }

    #[tool(
        description = "Send a message to the AI agent with access to MCP tools. The agent can use tools like kubernetes, ssh, github, sysinfo to accomplish tasks. Use 'servers' to filter to specific tool sets (recommended for smaller models). Use 'session_id' to maintain conversation across calls. Note: First call may take a few seconds to initialize connections."
    )]
    async fn agent_chat(
        &self,
        Parameters(params): Parameters<AgentChatParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(
            "agent_chat: {} (servers: {:?}, session: {:?})",
            params.message,
            params.servers,
            params.session_id
        );

        // Lazily initialize agent on first call
        self.ensure_agent().await?;

        let mut agent_guard = self.agent.lock().await;
        let agent = agent_guard.as_mut().unwrap(); // Safe: ensure_agent guarantees it's Some

        // Load session history or clear for stateless calls
        if let Some(ref session_id) = params.session_id {
            // Scope the lock to release it before chat
            let history_to_restore = {
                let sessions = self.sessions.lock().await;
                sessions.get(session_id).cloned()
            };
            if let Some(history) = history_to_restore {
                tracing::info!(
                    "Restoring session '{}' with {} messages",
                    session_id,
                    history.len()
                );
                agent.set_history(history);
            } else {
                tracing::info!("Creating new session '{}'", session_id);
                agent.clear_history();
            }
        } else {
            // No session_id = stateless, clear history
            agent.clear_history();
        }

        // Apply system prompt if provided (or clear it if not)
        if let Some(ref prompt) = params.system_prompt {
            tracing::info!("Setting system prompt: {}", prompt);
            agent.set_system_prompt(Some(prompt.clone()));
        } else {
            // Clear any previous system prompt for clean state
            agent.set_system_prompt(None);
        }

        // Use filtered servers if provided, otherwise use all tools
        let response = if let Some(ref servers) = params.servers {
            let server_refs: Vec<&str> = servers.iter().map(|s| s.as_str()).collect();
            agent.chat_with_servers(&params.message, &server_refs).await
        } else {
            agent.chat(&params.message).await
        };

        let response = response.map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // Save session history if session_id provided
        if let Some(ref session_id) = params.session_id {
            let mut sessions = self.sessions.lock().await;
            sessions.insert(session_id.clone(), agent.get_history());
            tracing::info!(
                "Saved session '{}' with {} messages",
                session_id,
                agent.get_history().len()
            );
        }

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    #[tool(description = "Clear a specific session's conversation history.")]
    async fn clear_session(
        &self,
        Parameters(params): Parameters<ClearSessionParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("clear_session: {}", params.session_id);

        let mut sessions = self.sessions.lock().await;
        if sessions.remove(&params.session_id).is_some() {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "Session '{}' cleared",
                params.session_id
            ))]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "Session '{}' not found (already cleared or never existed)",
                params.session_id
            ))]))
        }
    }

    #[tool(
        description = "List all available MCP tools that the agent can use. Note: First call may take a few seconds to initialize connections."
    )]
    async fn list_tools(
        &self,
        Parameters(params): Parameters<ListToolsParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("list_tools: server={:?}", params.server);

        // Lazily initialize agent on first call
        self.ensure_agent().await?;

        let mut guard = self.agent.lock().await;
        let agent = guard.as_mut().unwrap(); // Safe: ensure_agent guarantees it's Some

        let tools = agent
            .tool_names()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // Filter by server if specified (tool names are prefixed with server name)
        let tools: Vec<_> = match params.server {
            Some(ref server) => tools
                .into_iter()
                .filter(|t| t.starts_with(server))
                .collect(),
            None => tools,
        };

        let output = tools.join("\n");
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    // ========================================================================
    // Run Analysis Tools
    // ========================================================================

    /// Helper to ensure database is available
    fn ensure_db(&self) -> Result<&Database, McpError> {
        self.db.as_ref().ok_or_else(|| {
            McpError::internal_error(
                "Run tracking is not available. Enable with `enable_runs: true` in config."
                    .to_string(),
                None,
            )
        })
    }

    /// Helper to find run by ID prefix
    fn find_run_by_prefix(&self, prefix: &str) -> Result<Run, McpError> {
        let db = self.ensure_db()?;

        // First try exact match
        if let Ok(Some(run)) = db.get_run(prefix) {
            return Ok(run);
        }

        // Then try prefix match
        let filter = RunFilter {
            limit: Some(100),
            ..Default::default()
        };
        let runs = db
            .list_runs(&filter)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let matches: Vec<_> = runs.iter().filter(|r| r.id.starts_with(prefix)).collect();

        match matches.len() {
            0 => Err(McpError::invalid_params(
                format!("No run found matching prefix: {}", prefix),
                None,
            )),
            1 => {
                // Get full run data
                db.get_run(&matches[0].id)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
                    .ok_or_else(|| McpError::internal_error("Run disappeared".to_string(), None))
            }
            _ => Err(McpError::invalid_params(
                format!(
                    "Multiple runs match prefix '{}': {}",
                    prefix,
                    matches
                        .iter()
                        .map(|r| &r.id[..8])
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                None,
            )),
        }
    }

    #[tool(description = "List workflow runs with optional filtering by workflow name and status.")]
    async fn list_runs(
        &self,
        Parameters(params): Parameters<ListRunsParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(
            "list_runs: workflow={:?}, status={:?}, limit={:?}",
            params.workflow,
            params.status,
            params.limit
        );

        let db = self.ensure_db()?;

        let status = params
            .status
            .as_ref()
            .map(|s| {
                s.parse::<RunStatus>()
                    .map_err(|e| McpError::invalid_params(e.to_string(), None))
            })
            .transpose()?;

        let filter = RunFilter {
            workflow_name: params.workflow,
            status,
            limit: Some(params.limit.unwrap_or(20)),
            ..Default::default()
        };

        let runs = db
            .list_runs(&filter)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if runs.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No runs found matching the filter criteria.",
            )]));
        }

        // Format as a table
        let mut output = String::from("| ID | Workflow | Status | Duration | Started |\n");
        output.push_str("|----------|----------|----------|----------|----------|\n");

        for run in &runs {
            let duration = run
                .duration_ms
                .map(format_duration)
                .unwrap_or_else(|| "running".to_string());
            let started = run.started_at.format("%Y-%m-%d %H:%M").to_string();

            output.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                &run.id[..8],
                run.workflow_name,
                run.status,
                duration,
                started
            ));
        }

        output.push_str(&format!("\nTotal: {} runs", runs.len()));

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Get detailed information about a specific run, optionally including events and metrics."
    )]
    async fn get_run(
        &self,
        Parameters(params): Parameters<GetRunParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(
            "get_run: id={}, events={:?}, metrics={:?}",
            params.id,
            params.include_events,
            params.include_metrics
        );

        let db = self.ensure_db()?;
        let run = self.find_run_by_prefix(&params.id)?;

        let mut output = serde_json::json!({
            "id": run.id,
            "workflow": run.workflow_name,
            "task": run.task,
            "status": run.status.to_string(),
            "model": run.model,
            "started_at": run.started_at.to_rfc3339(),
            "completed_at": run.completed_at.map(|t| t.to_rfc3339()),
            "duration_ms": run.duration_ms,
            "error": run.error,
        });

        if params.include_events.unwrap_or(false) {
            let events = db
                .get_run_events(&run.id)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            output["events"] = serde_json::to_value(&events).unwrap_or_default();
            output["event_count"] = serde_json::json!(events.len());
        }

        if params.include_metrics.unwrap_or(false) {
            let metrics = db
                .get_run_metrics(&run.id)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            if let Some(m) = metrics {
                output["metrics"] = serde_json::to_value(&m).unwrap_or_default();
            }
        }

        let json_str = serde_json::to_string_pretty(&output)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json_str)]))
    }

    #[tool(
        description = "Export a run as a detailed analysis report in markdown or JSON format for review."
    )]
    async fn export_run(
        &self,
        Parameters(params): Parameters<ExportRunParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("export_run: id={}, format={:?}", params.id, params.format);

        let db = self.ensure_db()?;
        let run = self.find_run_by_prefix(&params.id)?;

        let events = db
            .get_run_events(&run.id)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let metrics = db
            .get_run_metrics(&run.id)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let format = params.format.as_deref().unwrap_or("markdown");

        let output = match format {
            "json" => {
                let export = serde_json::json!({
                    "run": run,
                    "events": events,
                    "metrics": metrics,
                });
                serde_json::to_string_pretty(&export)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?
            }
            _ => export_markdown(&run, &events, metrics.as_ref()),
        };

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "List recorded improvements and their impact on workflow execution.")]
    async fn list_improvements(
        &self,
        Parameters(params): Parameters<ListImprovementsParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(
            "list_improvements: category={:?}, status={:?}, limit={:?}",
            params.category,
            params.status,
            params.limit
        );

        let db = self.ensure_db()?;

        let category = params
            .category
            .as_ref()
            .map(|c| {
                c.parse()
                    .map_err(|e: anyhow::Error| McpError::invalid_params(e.to_string(), None))
            })
            .transpose()?;

        let status = params
            .status
            .as_ref()
            .map(|s| {
                s.parse()
                    .map_err(|e: anyhow::Error| McpError::invalid_params(e.to_string(), None))
            })
            .transpose()?;

        let filter = ImprovementFilter {
            category,
            status,
            limit: Some(params.limit.unwrap_or(20)),
            ..Default::default()
        };

        let improvements = db
            .list_improvements(&filter)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if improvements.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No improvements found matching the filter criteria.",
            )]));
        }

        let mut output = String::from("| ID | Category | Status | Description | Created |\n");
        output.push_str("|----------|----------|----------|----------|----------|\n");

        for imp in &improvements {
            let created = imp.created_at.format("%Y-%m-%d").to_string();
            let desc = if imp.description.len() > 40 {
                format!("{}...", &imp.description[..37])
            } else {
                imp.description.clone()
            };

            output.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                &imp.id[..8],
                imp.category,
                imp.status,
                desc,
                created
            ));
        }

        output.push_str(&format!("\nTotal: {} improvements", improvements.len()));

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Format duration in milliseconds to human-readable string
fn format_duration(ms: i64) -> String {
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes % 60)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds % 60)
    } else if seconds > 0 {
        format!("{}s", seconds)
    } else {
        format!("{}ms", ms)
    }
}

/// Export run as markdown analysis report
fn export_markdown(run: &Run, events: &[RunEvent], metrics: Option<&RunMetrics>) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&format!("# Run Analysis: {}\n\n", &run.id[..8]));

    // Overview
    output.push_str("## Overview\n\n");
    output.push_str(&format!("- **Workflow:** {}\n", run.workflow_name));
    output.push_str(&format!("- **Task:** {}\n", run.task));
    output.push_str(&format!("- **Status:** {}\n", run.status));
    if let Some(ms) = run.duration_ms {
        output.push_str(&format!("- **Duration:** {}\n", format_duration(ms)));
    }
    output.push_str(&format!("- **Model:** {}\n", run.model));
    output.push_str(&format!(
        "- **Started:** {}\n",
        run.started_at.format("%Y-%m-%d %H:%M:%S")
    ));
    if let Some(completed) = run.completed_at {
        output.push_str(&format!(
            "- **Completed:** {}\n",
            completed.format("%Y-%m-%d %H:%M:%S")
        ));
    }
    output.push('\n');

    // Error (if any)
    if let Some(ref error) = run.error {
        output.push_str("## Error\n\n");
        output.push_str(&format!("```\n{}\n```\n\n", error));
    }

    // Metrics
    if let Some(m) = metrics {
        output.push_str("## Metrics\n\n");
        output.push_str(&format!("- **Total Tool Calls:** {}\n", m.total_tool_calls));
        output.push_str(&format!("- **Successful:** {}\n", m.successful_tool_calls));
        output.push_str(&format!("- **Failed:** {}\n", m.failed_tool_calls));
        output.push_str(&format!("- **Files Read:** {}\n", m.files_read));
        output.push_str(&format!("- **Files Modified:** {}\n", m.files_modified));
        if let (Some(tokens_in), Some(tokens_out)) = (m.total_tokens_in, m.total_tokens_out) {
            output.push_str(&format!(
                "- **Tokens (in/out):** {} / {}\n",
                tokens_in, tokens_out
            ));
        }
        output.push('\n');
    }

    // Events Summary
    if !events.is_empty() {
        output.push_str("## Events\n\n");

        // Group by step
        let mut current_step: Option<usize> = None;
        for event in events {
            if current_step != Some(event.step_index) {
                current_step = Some(event.step_index);
                output.push_str(&format!("\n### Step {}\n\n", event.step_index + 1));
            }

            let timestamp = event.timestamp.format("%H:%M:%S").to_string();
            output.push_str(&format!("- `{}` **{}**: ", timestamp, event.event_type));

            // Extract meaningful info from event data (already a Value)
            if let Some(name) = event.event_data.get("name") {
                output.push_str(name.as_str().unwrap_or("?"));
            }
            if let Some(is_error) = event.event_data.get("is_error") {
                if is_error.as_bool().unwrap_or(false) {
                    output.push_str(" [ERROR]");
                }
            }
            output.push('\n');
        }
        output.push('\n');
    }

    // Context (if available and non-empty)
    if let Some(ref context) = run.context {
        // Check if context is a non-empty object or array
        let has_content = match context {
            serde_json::Value::Object(map) => !map.is_empty(),
            serde_json::Value::Array(arr) => !arr.is_empty(),
            serde_json::Value::Null => false,
            _ => true,
        };
        if has_content {
            output.push_str("## Context\n\n");
            output.push_str("```json\n");
            if let Ok(pretty) = serde_json::to_string_pretty(&context) {
                output.push_str(&pretty);
            } else {
                output.push_str(&context.to_string());
            }
            output.push_str("\n```\n\n");
        }
    }

    // Analysis Questions
    output.push_str("## Analysis Questions\n\n");
    output.push_str("1. Were there any unexpected tool failures?\n");
    output.push_str("2. Could the workflow be optimized to reduce duration?\n");
    output.push_str("3. Were the right tools used for each step?\n");
    output.push_str("4. Any patterns that suggest prompt improvements?\n");

    output
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for AgentMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Agent MCP Server - provides AI assistant capabilities with access to MCP tools. \
                 Use 'chat' for simple LLM interactions, or 'agent_chat' for tool-assisted tasks."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// Start the MCP server on stdio
pub async fn serve(config: ServerConfig) -> anyhow::Result<()> {
    use rmcp::{transport::stdio, ServiceExt};

    tracing::info!("Starting Agent MCP Server");
    tracing::info!("Model: {}", config.model);

    // Create the server - agent initialization is now lazy
    // (happens on first agent_chat call, not at startup)
    let server = AgentMcpServer::new(config);

    // Create stdio transport and serve immediately
    // Don't block on MCP server connections - that would timeout Claude
    let service = server.serve(stdio()).await?;

    tracing::info!("Server running on stdio, waiting for requests...");

    // Wait for shutdown
    service.waiting().await?;

    tracing::info!("Server shutting down");
    Ok(())
}
