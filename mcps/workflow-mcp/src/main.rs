//! Workflow MCP Server
//!
//! This MCP server provides workflow orchestration capabilities:
//! - Execute multi-step agent workflows
//! - Handle checkpoints for human approval
//! - Track execution state and resume from checkpoints
//! - Load built-in and custom workflow definitions

use workflow_mcp::server::WorkflowMcpServer;

mcp_common::serve_stdio!(WorkflowMcpServer, "workflow_mcp");
