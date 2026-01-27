//! Filesystem MCP - Sandboxed filesystem server with security controls
//!
//! Provides secure file operations with configurable allowlists/denylists.
//! Operations are restricted to configured directories.

mod handlers;
mod params;
mod sandbox;
mod server;
mod types;

use server::FilesystemMcpServer;

mcp_common::serve_stdio!(FilesystemMcpServer, "filesystem_mcp");
