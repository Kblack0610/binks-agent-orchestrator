//! Exec MCP Server binary entry point

use exec_mcp::ExecMcpServer;

mcp_common::serve_stdio!(ExecMcpServer, "exec_mcp");
