//! Inbox MCP Server binary entry point

use inbox_mcp::InboxMcpServer;

mcp_common::serve_stdio!(InboxMcpServer, "inbox_mcp");
