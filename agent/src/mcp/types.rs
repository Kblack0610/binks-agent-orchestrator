//! MCP type definitions
//!
//! Shared types used across MCP client implementations.

use serde_json::Value;

/// A tool from an MCP server
#[derive(Debug, Clone)]
pub struct McpTool {
    /// Server this tool belongs to
    pub server: String,
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: Option<String>,
    /// Input schema (JSON)
    pub input_schema: Option<Value>,
}
