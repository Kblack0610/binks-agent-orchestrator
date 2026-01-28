//! SQL MCP Server
//!
//! Provides database query tools for SQL databases.
//! Read-only by default, write mode can be enabled via config.

mod config;
mod server;

use server::SqlMcpServer;

mcp_common::serve_stdio!(SqlMcpServer, "sql_mcp");
