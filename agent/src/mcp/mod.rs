//! MCP (Model Context Protocol) client implementation
//!
//! Connects to MCP servers defined in .mcp.json and provides access to their tools.

mod client;

pub use client::{McpClient, McpClientPool, McpTool};
