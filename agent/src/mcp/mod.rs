//! MCP (Model Context Protocol) client implementation
//!
//! Connects to MCP servers defined in .mcp.json and provides access to their tools.
//!
//! Three modes are supported:
//! 1. Daemon mode (preferred) - persistent connections via Unix socket
//! 2. `McpClientPool` - Send-safe, uses spawn-per-call with tool caching
//! 3. `McpConnectionManager` - Channel-based manager for persistent connections

mod manager;
pub mod model_size;
mod pool;
mod spawn;
mod types;

pub use manager::{McpConnectionManager, McpManagerHandle};
pub use model_size::{parse_model_size, parse_model_size_with_thresholds, ModelSize};
pub use pool::McpClientPool;
pub use spawn::McpClient;
pub use types::McpTool;
