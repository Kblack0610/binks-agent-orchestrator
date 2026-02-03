//! Linear CLI MCP Server
//!
//! A lightweight MCP server wrapping the `linear` CLI for Linear issue tracking.
//!
//! # Features
//! - Issues: List, view, create, start, comment, get ID from branch
//! - Teams: List, members
//! - Projects: List
//! - Documents: List, view
//!
//! # Requirements
//! - `linear` CLI installed (`brew install schpet/tap/linear`)

mod handlers;
mod linear;
mod params;
mod server;

use server::LinearCliMcpServer;

mcp_common::serve_stdio!(LinearCliMcpServer, "linear_cli_mcp");
