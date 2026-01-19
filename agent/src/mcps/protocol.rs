//! IPC Protocol for MCP Daemon communication
//!
//! Defines the message types exchanged between the agent and the MCP daemon
//! over Unix sockets.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request from agent to daemon
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonRequest {
    /// List all available tools from a specific server
    ListTools { server: String },

    /// List all tools from all servers
    ListAllTools,

    /// Call a tool on a specific server
    CallTool {
        server: String,
        tool: String,
        arguments: Option<Value>,
    },

    /// Get status of all managed MCP servers
    Status,

    /// Refresh a specific server (restart and clear cache)
    RefreshServer { server: String },

    /// Refresh all servers
    RefreshAll,

    /// Shutdown the daemon
    Shutdown,

    /// Ping to check if daemon is alive
    Ping,
}

/// Response from daemon to agent
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonResponse {
    /// List of tools
    Tools { tools: Vec<ToolInfo> },

    /// Tool call result
    ToolResult { result: ToolCallResult },

    /// Server status information
    Status { servers: Vec<ServerStatus> },

    /// Simple success acknowledgment
    Ok,

    /// Pong response
    Pong,

    /// Error response
    Error { message: String },
}

/// Information about a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub server: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

/// Result of a tool call
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<ToolContent>,
    pub is_error: bool,
}

/// Content from a tool call
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

/// Status of a managed MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub name: String,
    pub state: ServerState,
    pub tool_count: usize,
    pub last_used_secs: Option<u64>,
    pub uptime_secs: Option<u64>,
}

/// State of an MCP server
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServerState {
    /// Server is running and connected
    Running,
    /// Server is starting up
    Starting,
    /// Server failed to start or crashed
    Failed,
    /// Server was stopped
    Stopped,
    /// Server not yet started
    Idle,
}

impl std::fmt::Display for ServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerState::Running => write!(f, "running"),
            ServerState::Starting => write!(f, "starting"),
            ServerState::Failed => write!(f, "failed"),
            ServerState::Stopped => write!(f, "stopped"),
            ServerState::Idle => write!(f, "idle"),
        }
    }
}

/// Default socket path for the MCP daemon
pub fn default_socket_path() -> std::path::PathBuf {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("binks-agent");
    cache_dir.join("mcps.sock")
}

/// PID file path for the daemon
pub fn default_pid_path() -> std::path::PathBuf {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("binks-agent");
    cache_dir.join("mcps.pid")
}

/// Log directory for daemon logs
pub fn default_log_dir() -> std::path::PathBuf {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("binks-agent");
    cache_dir.join("logs")
}
