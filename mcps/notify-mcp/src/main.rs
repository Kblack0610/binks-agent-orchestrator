//! Notify MCP Server binary entry point

use notify_mcp::NotifyMcpServer;

mcp_common::serve_stdio!(NotifyMcpServer, "notify_mcp");
