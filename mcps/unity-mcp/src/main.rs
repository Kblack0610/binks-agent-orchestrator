//! Unity MCP Server binary entry point

use unity_mcp::UnityMcpServer;

mcp_common::serve_stdio!(UnityMcpServer, "unity_mcp");
