//! System Info MCP Library
//!
//! Cross-platform system information tools via MCP.
//! Retrieves OS details, CPU/memory stats, disk usage, network interfaces, and uptime.
//!
//! # Usage as Library
//!
//! ```rust,ignore
//! use sysinfo_mcp::SysInfoMcpServer;
//!
//! let server = SysInfoMcpServer::new();
//! // Use with in-memory transport or serve via stdio
//! ```
//!
//! # Usage as Binary
//!
//! Run directly: `sysinfo-mcp`
//!
//! Or configure in `.mcp.json`:
//! ```json
//! { "mcpServers": { "sysinfo": { "command": "./sysinfo-mcp" } } }
//! ```

pub mod info;
pub mod server;
pub mod types;

// Re-export main server type
pub use server::SysInfoMcpServer;

// Re-export parameter types for direct API usage
pub use server::{CpuInfoParams, CpuUsageParams, DiskInfoParams, NetworkParams};

// Re-export EmbeddableMcp trait for in-process usage
pub use mcp_common::{EmbeddableMcp, EmbeddableError, EmbeddableResult};
