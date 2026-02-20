use adb_mcp::AdbMcpServer;

mcp_common::serve_stdio!(AdbMcpServer, "adb_mcp");
