mod doctl;
mod handlers;
mod params;
mod server;

use server::DoctlMcpServer;

mcp_common::serve_stdio!(DoctlMcpServer, "doctl_mcp");
