//! ADB tool handler implementations
//!
//! Each handler takes parameters and returns MCP results using mcp-common helpers.

use mcp_common::{
    internal_error, invalid_params, json_success, text_success, CallToolResult, Content, McpError,
};

use crate::adb;
use crate::params::*;
use crate::processing::{CropRect, ProcessOptions};

/// Resolve a device serial, returning an MCP error on failure
async fn resolve_device(device: Option<&str>) -> Result<String, McpError> {
    adb::get_device(device)
        .await
        .map_err(|e| internal_error(format!("Device error: {e}")))
}

pub async fn devices(_params: DevicesParams) -> Result<CallToolResult, McpError> {
    let devices = adb::list_devices()
        .await
        .map_err(|e| internal_error(format!("Failed to list devices: {e}")))?;
    json_success(&devices)
}

pub async fn screenshot(params: ScreenshotParams) -> Result<CallToolResult, McpError> {
    let device = resolve_device(params.device.as_deref()).await?;

    // Validate format early
    let format = params.format.as_deref().unwrap_or("jpeg");
    if !matches!(format, "jpeg" | "jpg" | "png") {
        return Err(invalid_params(format!(
            "Unsupported format \"{format}\", use \"jpeg\" or \"png\""
        )));
    }

    // Wake device first
    if let Err(e) = adb::wake_device(&device).await {
        tracing::warn!("Failed to wake device: {}", e);
    }

    let result = adb::capture_screenshot(&device)
        .await
        .map_err(|e| internal_error(format!("Screenshot capture failed: {e}")))?;

    // Build processing options with defaults
    let opts = ProcessOptions {
        format: format.to_string(),
        quality: params.quality.unwrap_or(80),
        max_width: params.max_width.unwrap_or(1024),
        max_height: params.max_height.unwrap_or(1920),
        crop: params.region.map(|r| CropRect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
        }),
    };

    let processed = crate::processing::process_screenshot(&result.data, &opts)
        .map_err(|e| internal_error(format!("Image processing failed: {e}")))?;

    if let Some(path) = params.output_path {
        tokio::fs::write(&path, &processed.data)
            .await
            .map_err(|e| internal_error(format!("Failed to save screenshot: {e}")))?;
        Ok(text_success(format!(
            "Screenshot saved to {} ({}x{}, {} bytes, {})",
            path,
            processed.width,
            processed.height,
            processed.data.len(),
            processed.mime_type
        )))
    } else {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&processed.data);
        Ok(CallToolResult::success(vec![Content::image(
            &b64,
            processed.mime_type,
        )]))
    }
}

pub async fn tap(params: TapParams) -> Result<CallToolResult, McpError> {
    let device = resolve_device(params.device.as_deref()).await?;
    adb::tap(&device, params.x, params.y)
        .await
        .map_err(|e| internal_error(format!("Tap failed: {e}")))?;
    Ok(text_success(format!(
        "Tapped at ({}, {})",
        params.x, params.y
    )))
}

pub async fn swipe(params: SwipeParams) -> Result<CallToolResult, McpError> {
    let device = resolve_device(params.device.as_deref()).await?;
    adb::swipe(
        &device,
        params.start_x,
        params.start_y,
        params.end_x,
        params.end_y,
        params.duration_ms,
    )
    .await
    .map_err(|e| internal_error(format!("Swipe failed: {e}")))?;
    Ok(text_success(format!(
        "Swiped from ({}, {}) to ({}, {})",
        params.start_x, params.start_y, params.end_x, params.end_y
    )))
}

pub async fn input_text(params: InputTextParams) -> Result<CallToolResult, McpError> {
    let device = resolve_device(params.device.as_deref()).await?;
    adb::input_text(&device, &params.text)
        .await
        .map_err(|e| internal_error(format!("Input text failed: {e}")))?;
    Ok(text_success(format!("Input text: {}", params.text)))
}

pub async fn keyevent(params: KeyeventParams) -> Result<CallToolResult, McpError> {
    let device = resolve_device(params.device.as_deref()).await?;
    adb::keyevent(&device, &params.key)
        .await
        .map_err(|e| internal_error(format!("Keyevent failed: {e}")))?;
    Ok(text_success(format!("Sent key: {}", params.key)))
}

pub async fn shell(params: ShellParams) -> Result<CallToolResult, McpError> {
    let device = resolve_device(params.device.as_deref()).await?;
    let output = adb::shell(&device, &params.command)
        .await
        .map_err(|e| internal_error(format!("Shell command failed: {e}")))?;

    let mut result = output.stdout;
    if !output.stderr.is_empty() {
        result.push_str("\n[stderr]: ");
        result.push_str(&output.stderr);
    }
    Ok(text_success(result))
}

pub async fn ui_dump(params: UiDumpParams) -> Result<CallToolResult, McpError> {
    let device = resolve_device(params.device.as_deref()).await?;
    let elements = adb::dump_ui(&device)
        .await
        .map_err(|e| internal_error(format!("UI dump failed: {e}")))?;
    json_success(&elements)
}

pub async fn find_element(params: FindElementParams) -> Result<CallToolResult, McpError> {
    let device = resolve_device(params.device.as_deref()).await?;
    let elements = adb::dump_ui(&device)
        .await
        .map_err(|e| internal_error(format!("Find element failed: {e}")))?;

    let found = adb::find_elements(
        &elements,
        params.text.as_deref(),
        params.resource_id.as_deref(),
        params.class.as_deref(),
    );
    json_success(&found)
}

pub async fn tap_element(params: TapElementParams) -> Result<CallToolResult, McpError> {
    let device = resolve_device(params.device.as_deref()).await?;

    let elements = adb::dump_ui(&device)
        .await
        .map_err(|e| internal_error(format!("UI dump failed: {e}")))?;

    let found = adb::find_elements(
        &elements,
        params.text.as_deref(),
        params.resource_id.as_deref(),
        params.class.as_deref(),
    );

    if found.is_empty() {
        return Err(internal_error("No matching element found"));
    }

    let element = found[0];
    let (x, y) = element.bounds.center();

    adb::tap(&device, x, y)
        .await
        .map_err(|e| internal_error(format!("Tap failed: {e}")))?;

    Ok(text_success(format!(
        "Tapped element at ({}, {}) - text: {:?}, id: {:?}",
        x, y, element.text, element.resource_id
    )))
}

pub async fn get_current_activity(params: GetActivityParams) -> Result<CallToolResult, McpError> {
    let device = resolve_device(params.device.as_deref()).await?;
    let activity = adb::get_current_activity(&device)
        .await
        .map_err(|e| internal_error(format!("Failed to get activity: {e}")))?;
    Ok(text_success(activity))
}

pub async fn wait_for_activity(params: WaitForActivityParams) -> Result<CallToolResult, McpError> {
    let device = resolve_device(params.device.as_deref()).await?;
    let timeout = params.timeout_ms.unwrap_or(10000);

    match adb::wait_for_activity(&device, &params.activity, timeout).await {
        Ok(true) => Ok(text_success(format!(
            "Activity '{}' appeared",
            params.activity
        ))),
        Ok(false) => Err(internal_error(format!(
            "Timeout waiting for activity '{}'",
            params.activity
        ))),
        Err(e) => Err(internal_error(format!("Wait failed: {e}"))),
    }
}
