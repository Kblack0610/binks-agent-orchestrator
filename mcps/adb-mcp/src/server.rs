//! MCP Server implementation for Android Debug Bridge operations

use mcp_common::{CallToolResult, McpError};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::handlers;
use crate::params::*;

/// The ADB MCP Server
#[derive(Clone)]
pub struct AdbMcpServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AdbMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List all connected ADB devices with serial numbers and model info")]
    async fn adb_devices(
        &self,
        Parameters(params): Parameters<DevicesParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::devices(params).await
    }

    #[tool(description = "Capture a validated screenshot from an Android device. Returns base64-encoded PNG or saves to file.")]
    async fn adb_screenshot(
        &self,
        Parameters(params): Parameters<ScreenshotParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::screenshot(params).await
    }

    #[tool(description = "Tap at specific x,y coordinates on the device screen")]
    async fn adb_tap(
        &self,
        Parameters(params): Parameters<TapParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::tap(params).await
    }

    #[tool(description = "Perform a swipe gesture from start to end coordinates")]
    async fn adb_swipe(
        &self,
        Parameters(params): Parameters<SwipeParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::swipe(params).await
    }

    #[tool(description = "Type text on the device (requires focus on a text field)")]
    async fn adb_input_text(
        &self,
        Parameters(params): Parameters<InputTextParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::input_text(params).await
    }

    #[tool(description = "Send a key event (e.g., BACK, HOME, ENTER, or numeric keycode)")]
    async fn adb_keyevent(
        &self,
        Parameters(params): Parameters<KeyeventParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::keyevent(params).await
    }

    #[tool(description = "Execute an arbitrary shell command on the device")]
    async fn adb_shell(
        &self,
        Parameters(params): Parameters<ShellParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::shell(params).await
    }

    #[tool(description = "Dump the current UI hierarchy as JSON (useful for finding elements)")]
    async fn adb_ui_dump(
        &self,
        Parameters(params): Parameters<UiDumpParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::ui_dump(params).await
    }

    #[tool(description = "Find UI elements by text, resource ID, or class name")]
    async fn adb_find_element(
        &self,
        Parameters(params): Parameters<FindElementParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::find_element(params).await
    }

    #[tool(description = "Find a UI element and tap on it. Combines element finding with tap action.")]
    async fn adb_tap_element(
        &self,
        Parameters(params): Parameters<TapElementParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::tap_element(params).await
    }

    #[tool(description = "Get the currently focused activity/app")]
    async fn adb_get_current_activity(
        &self,
        Parameters(params): Parameters<GetActivityParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::get_current_activity(params).await
    }

    #[tool(description = "Wait for a specific activity to appear on screen (useful for navigation timing)")]
    async fn adb_wait_for_activity(
        &self,
        Parameters(params): Parameters<WaitForActivityParams>,
    ) -> Result<CallToolResult, McpError> {
        handlers::wait_for_activity(params).await
    }
}

#[tool_handler]
impl rmcp::ServerHandler for AdbMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "ADB MCP server for Android device automation. \
                 Provides screenshot capture with PNG validation, \
                 touch/swipe input, UI hierarchy inspection, and shell access."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for AdbMcpServer {
    fn default() -> Self {
        Self::new()
    }
}
