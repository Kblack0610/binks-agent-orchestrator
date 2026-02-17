//! SQL MCP Server binary entry point

use sql_mcp::SqlMcpServer;

mcp_common::serve_stdio!(SqlMcpServer, "sql_mcp");
