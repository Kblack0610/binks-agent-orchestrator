//! MCP Server implementation for Unity log monitoring

use mcp_common::{
    async_trait, json_success, EmbeddableError, EmbeddableMcp, EmbeddableResult, McpError,
};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo, Tool},
    tool, tool_handler, tool_router,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::handlers::{logs, project};
use crate::params::*;

/// The main Unity MCP Server
#[derive(Clone)]
pub struct UnityMcpServer {
    /// Byte offset for log tailing (tracks position between calls)
    tail_offset: Arc<Mutex<u64>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl UnityMcpServer {
    pub fn new() -> Self {
        Self {
            tail_offset: Arc::new(Mutex::new(0)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Read recent Unity Editor.log entries, optionally filtered by level (error/warning/info) or regex pattern. Returns structured entries with timestamps when available."
    )]
    async fn unity_read_log(
        &self,
        Parameters(params): Parameters<ReadLogParams>,
    ) -> Result<CallToolResult, McpError> {
        let lines = params.lines.unwrap_or(100);
        let resp = logs::read_log(
            lines,
            params.level.as_deref(),
            params.pattern.as_deref(),
            params.log_path.as_deref(),
        )?;
        json_success(&resp)
    }

    #[tool(
        description = "Parse Unity Editor.log for compile errors and exceptions. Returns structured list with file path, line number, column, error code, and message."
    )]
    async fn unity_log_errors(
        &self,
        Parameters(params): Parameters<LogErrorsParams>,
    ) -> Result<CallToolResult, McpError> {
        let resp = logs::log_errors(params.log_path.as_deref())?;
        json_success(&resp)
    }

    #[tool(
        description = "Return new Unity Editor.log entries since last call. Tracks file position between calls for efficient polling during workflows. Handles log rotation automatically."
    )]
    async fn unity_log_tail(
        &self,
        Parameters(params): Parameters<LogTailParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut offset = self.tail_offset.lock().await;
        let (resp, new_offset) = logs::log_tail(*offset, params.log_path.as_deref())?;
        *offset = new_offset;
        json_success(&resp)
    }

    #[tool(
        description = "Detect and return Unity project information: project path, Unity version from ProjectVersion.txt, and package dependencies from manifest.json."
    )]
    async fn unity_project_info(
        &self,
        Parameters(params): Parameters<ProjectInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        let resp = project::project_info(params.project_path.as_deref())?;
        json_success(&resp)
    }
}

#[tool_handler]
impl rmcp::ServerHandler for UnityMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Unity Editor log monitoring and project analysis MCP server. \
                 Reads Unity state entirely from the filesystem — no Editor plugin required."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for UnityMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmbeddableMcp for UnityMcpServer {
    fn server_name(&self) -> &str {
        "unity"
    }

    fn server_description(&self) -> Option<&str> {
        Some(
            "Unity Editor log monitoring and project analysis MCP server. \
             Reads Unity state entirely from the filesystem — no Editor plugin required.",
        )
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tool_router.list_all()
    }

    async fn call_tool(&self, name: &str, params: Value) -> EmbeddableResult<CallToolResult> {
        match name {
            "unity_read_log" => {
                let p: ReadLogParams = serde_json::from_value(params)?;
                self.unity_read_log(Parameters(p)).await.map_err(Into::into)
            }
            "unity_log_errors" => {
                let p: LogErrorsParams = serde_json::from_value(params)?;
                self.unity_log_errors(Parameters(p))
                    .await
                    .map_err(Into::into)
            }
            "unity_log_tail" => {
                let p: LogTailParams = serde_json::from_value(params)?;
                self.unity_log_tail(Parameters(p))
                    .await
                    .map_err(Into::into)
            }
            "unity_project_info" => {
                let p: ProjectInfoParams = serde_json::from_value(params)?;
                self.unity_project_info(Parameters(p))
                    .await
                    .map_err(Into::into)
            }
            _ => Err(EmbeddableError::ToolNotFound(name.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_name() {
        let server = UnityMcpServer::new();
        assert_eq!(server.server_name(), "unity");
    }

    #[test]
    fn test_list_tools() {
        let server = UnityMcpServer::new();
        let tools = server.list_tools();
        assert_eq!(tools.len(), 4);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"unity_read_log"));
        assert!(names.contains(&"unity_log_errors"));
        assert!(names.contains(&"unity_log_tail"));
        assert!(names.contains(&"unity_project_info"));
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let server = UnityMcpServer::new();
        let result = server
            .call_tool("nonexistent", serde_json::json!({}))
            .await;
        assert!(matches!(result, Err(EmbeddableError::ToolNotFound(_))));
    }
}
