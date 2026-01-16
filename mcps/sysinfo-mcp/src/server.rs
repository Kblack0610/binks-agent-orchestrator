//! MCP Server implementation for system information
//!
//! This module defines the main MCP server that exposes system information
//! tools for querying OS, CPU, memory, disk, network, and uptime data.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::Mutex;

use crate::info;

/// The main System Info MCP Server
///
/// This server provides cross-platform system information tools for
/// querying hardware and software details of the host system.
#[derive(Clone)]
pub struct SysInfoMcpServer {
    /// Shared System instance for efficiency (sysinfo recommends reusing)
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

    // ========================================================================
    // OS Tools
    // ========================================================================

    #[tool(
        description = "Get operating system information including name, version, kernel version, hostname, and architecture"
    )]
    async fn get_os_info(&self) -> Result<CallToolResult, McpError> {
        let os_info = info::os::get_os_info();
        let json = serde_json::to_string_pretty(&os_info)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // CPU Tools
    // ========================================================================

    #[tool(
        description = "Get CPU hardware information including model, vendor, physical and logical core counts, and frequency"
    )]
    async fn get_cpu_info(
        &self,
        Parameters(params): Parameters<CpuInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sys = self.system.lock().await;
        sys.refresh_cpu_all();
        let cpu_info = info::cpu::get_cpu_info(&sys, params.include_per_core.unwrap_or(false));
        let json = serde_json::to_string_pretty(&cpu_info)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get current CPU usage percentage (global and optionally per-core)")]
    async fn get_cpu_usage(
        &self,
        Parameters(params): Parameters<CpuUsageParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sys = self.system.lock().await;
        // Need to refresh twice with delay for accurate usage measurement
        sys.refresh_cpu_usage();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        sys.refresh_cpu_usage();

        let usage = info::cpu::get_cpu_usage(&sys, params.per_core.unwrap_or(false));
        let json = serde_json::to_string_pretty(&usage)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Memory Tools
    // ========================================================================

    #[tool(
        description = "Get memory information including total, used, and available RAM, plus swap usage"
    )]
    async fn get_memory_info(&self) -> Result<CallToolResult, McpError> {
        let mut sys = self.system.lock().await;
        sys.refresh_memory();
        let mem_info = info::memory::get_memory_info(&sys);
        let json = serde_json::to_string_pretty(&mem_info)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Disk Tools
    // ========================================================================

    #[tool(
        description = "Get disk partition information including mount points, filesystem types, total/used/available space"
    )]
    async fn get_disk_info(
        &self,
        Parameters(params): Parameters<DiskInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        let disk_info = info::disk::get_disk_info(params.mount_point.as_deref());
        let json = serde_json::to_string_pretty(&disk_info)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Network Tools
    // ========================================================================

    #[tool(
        description = "Get network interface information including names, MAC addresses, IP addresses, and traffic statistics"
    )]
    async fn get_network_interfaces(
        &self,
        Parameters(params): Parameters<NetworkParams>,
    ) -> Result<CallToolResult, McpError> {
        let net_info = info::network::get_network_interfaces(params.interface.as_deref());
        let json = serde_json::to_string_pretty(&net_info)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Uptime Tools
    // ========================================================================

    #[tool(
        description = "Get system uptime in seconds and human-readable format, plus boot timestamp"
    )]
    async fn get_uptime(&self) -> Result<CallToolResult, McpError> {
        let uptime = info::uptime::get_uptime();
        let json = serde_json::to_string_pretty(&uptime)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // Summary Tools
    // ========================================================================

    #[tool(
        description = "Get a combined summary of all system information (OS, CPU, memory, disks, network, uptime)"
    )]
    async fn get_system_summary(&self) -> Result<CallToolResult, McpError> {
        let mut sys = self.system.lock().await;
        sys.refresh_all();
        // Brief delay for accurate CPU usage
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        sys.refresh_cpu_usage();

        let summary = info::get_system_summary(&sys);
        let json = serde_json::to_string_pretty(&summary)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
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
                 retrieving OS, CPU, memory, disk, network, and uptime information. \
                 Works on Linux, macOS, and Windows."
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
