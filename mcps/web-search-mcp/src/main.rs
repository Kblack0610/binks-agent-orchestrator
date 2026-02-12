//! Web Search MCP Server binary entry point

use web_search_mcp::WebSearchMcpServer;

mcp_common::serve_stdio!(WebSearchMcpServer, "web_search_mcp");
