//! System Info MCP Server
//!
//! Cross-platform system information tools via MCP.
//! Retrieves OS details, CPU/memory stats, disk usage, network interfaces, and uptime.
//!
//! # Usage
//!
//! Run directly: `sysinfo-mcp`
//!
//! Or configure in `.mcp.json`:
//! ```json
//! { "mcpServers": { "sysinfo": { "command": "./sysinfo-mcp" } } }
//! ```

mod info;
mod server;
mod types;

use server::SysInfoMcpServer;

mcp_common::serve_stdio!(SysInfoMcpServer, "sysinfo_mcp");
