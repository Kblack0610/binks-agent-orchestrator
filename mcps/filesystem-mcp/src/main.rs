//! Filesystem MCP Server binary entry point

use filesystem_mcp::FilesystemMcpServer;

mcp_common::serve_stdio!(FilesystemMcpServer, "filesystem_mcp");
