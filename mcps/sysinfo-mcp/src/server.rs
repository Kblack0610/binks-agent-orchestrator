//! MCP Server implementation for system information

use mcp_common::{json_success, McpError};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::Mutex;

use crate::info;

/// The main System Info MCP Server
#[derive(Clone)]
pub struct SysInfoMcpServer {
    system: Arc<Mutex<System>>,
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CpuInfoParams {
    #[schemars(description = "Include per-core information")]
    pub include_per_core: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CpuUsageParams {
    #[schemars(description = "Return usage per CPU core instead of just global usage")]
    pub per_core: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DiskInfoParams {
    #[schemars(description = "Filter results by mount point path (partial match)")]
    pub mount_point: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NetworkParams {
    #[schemars(description = "Filter results by interface name (partial match)")]
    pub interface: Option<String>,
}

// ============================================================================
// Tool Router Implementation
// ============================================================================

#[tool_router]
impl SysInfoMcpServer {
    pub fn new() -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new_all())),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Get operating system information including name, version, kernel version, hostname, and architecture"
    )]
    async fn get_os_info(&self) -> Result<CallToolResult, McpError> {
        json_success(&info::os::get_os_info())
    }

    #[tool(
        description = "Get CPU hardware information including model, vendor, physical and logical core counts, and frequency"
    )]
    async fn get_cpu_info(
        &self,
        Parameters(params): Parameters<CpuInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sys = self.system.lock().await;
        sys.refresh_cpu_all();
        json_success(&info::cpu::get_cpu_info(&sys, params.include_per_core.unwrap_or(false)))
    }

    #[tool(description = "Get current CPU usage percentage (global and optionally per-core)")]
    async fn get_cpu_usage(
        &self,
        Parameters(params): Parameters<CpuUsageParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sys = self.system.lock().await;
        sys.refresh_cpu_usage();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        sys.refresh_cpu_usage();
        json_success(&info::cpu::get_cpu_usage(&sys, params.per_core.unwrap_or(false)))
    }

    #[tool(
        description = "Get memory information including total, used, and available RAM, plus swap usage"
    )]
    async fn get_memory_info(&self) -> Result<CallToolResult, McpError> {
        let mut sys = self.system.lock().await;
        sys.refresh_memory();
        json_success(&info::memory::get_memory_info(&sys))
    }

    #[tool(
        description = "Get disk partition information including mount points, filesystem types, total/used/available space"
    )]
    async fn get_disk_info(
        &self,
        Parameters(params): Parameters<DiskInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        json_success(&info::disk::get_disk_info(params.mount_point.as_deref()))
    }

    #[tool(
        description = "Get network interface information including names, MAC addresses, IP addresses, and traffic statistics"
    )]
    async fn get_network_interfaces(
        &self,
        Parameters(params): Parameters<NetworkParams>,
    ) -> Result<CallToolResult, McpError> {
        json_success(&info::network::get_network_interfaces(params.interface.as_deref()))
    }

    #[tool(
        description = "Get system uptime in seconds and human-readable format, plus boot timestamp"
    )]
    async fn get_uptime(&self) -> Result<CallToolResult, McpError> {
        json_success(&info::uptime::get_uptime())
    }

    #[tool(
        description = "Get a combined summary of all system information (OS, CPU, memory, disks, network, uptime)"
    )]
    async fn get_system_summary(&self) -> Result<CallToolResult, McpError> {
        let mut sys = self.system.lock().await;
        sys.refresh_all();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        sys.refresh_cpu_usage();
        json_success(&info::get_system_summary(&sys))
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::ServerHandler for SysInfoMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Cross-platform System Information MCP Server - provides tools for \
                 retrieving OS, CPU, memory, disk, network, and uptime information."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for SysInfoMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
