//! Exec MCP - Sandboxed command execution server with security controls
//!
//! Provides shell command execution with configurable allow/deny lists,
//! timeout enforcement, and output size limits.

mod guard;
mod handlers;
mod params;
mod server;
mod types;

use server::ExecMcpServer;

mcp_common::serve_stdio!(ExecMcpServer, "exec_mcp");
