//! Linear CLI MCP Server binary entry point

use linear_cli_mcp::LinearCliMcpServer;

mcp_common::serve_stdio!(LinearCliMcpServer, "linear_cli_mcp");
