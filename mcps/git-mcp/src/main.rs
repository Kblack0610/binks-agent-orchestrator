//! Git MCP Server binary entry point

use git_mcp::GitMcpServer;

mcp_common::serve_stdio!(GitMcpServer, "git_mcp");
