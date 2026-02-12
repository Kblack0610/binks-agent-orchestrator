//! System Info MCP Server binary entry point

use sysinfo_mcp::SysInfoMcpServer;

mcp_common::serve_stdio!(SysInfoMcpServer, "sysinfo_mcp");
