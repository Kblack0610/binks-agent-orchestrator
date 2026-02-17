//! GitHub CLI MCP Server binary entry point

use github_gh_mcp::GitHubMcpServer;

mcp_common::serve_stdio!(GitHubMcpServer, "github_gh_mcp");
