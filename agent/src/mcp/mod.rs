//! MCP (Model Context Protocol) client implementation
//!
//! Connects to MCP servers defined in .mcp.json and provides access to their tools.
//!
//! Two modes are supported:
//! 1. `McpClientPool` - Send-safe, uses spawn-per-call with tool caching
//! 2. `McpConnectionManager` - Channel-based manager for persistent connections (Phase 2)

mod client;
pub mod model_size;

pub use client::{McpClient, McpClientPool, McpConnectionManager, McpManagerHandle, McpTool};
pub use model_size::{parse_model_size, parse_model_size_with_thresholds, ModelSize};
