//! Parameter types for ADB MCP tools

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DevicesParams {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CropRegion {
    #[schemars(description = "X offset of the crop region (pixels from left)")]
    pub x: u32,

    #[schemars(description = "Y offset of the crop region (pixels from top)")]
    pub y: u32,

    #[schemars(description = "Width of the crop region in pixels")]
    pub width: u32,

    #[schemars(description = "Height of the crop region in pixels")]
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ScreenshotParams {
    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,

    #[schemars(description = "File path to save screenshot (optional, returns base64 if omitted)")]
    #[serde(default)]
    pub output_path: Option<String>,

    #[schemars(description = "Output format: \"png\" or \"jpeg\" (default: \"jpeg\")")]
    #[serde(default)]
    pub format: Option<String>,

    #[schemars(description = "JPEG quality 1-100 (default: 80, ignored for PNG)")]
    #[serde(default)]
    pub quality: Option<u8>,

    #[schemars(
        description = "Max output width in pixels, preserving aspect ratio (default: 1024, 0 = no resize)"
    )]
    #[serde(default)]
    pub max_width: Option<u32>,

    #[schemars(description = "Crop region to extract before resizing")]
    #[serde(default)]
    pub region: Option<CropRegion>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TapParams {
    #[schemars(description = "X coordinate to tap")]
    pub x: i32,

    #[schemars(description = "Y coordinate to tap")]
    pub y: i32,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SwipeParams {
    #[schemars(description = "Starting X coordinate")]
    pub start_x: i32,

    #[schemars(description = "Starting Y coordinate")]
    pub start_y: i32,

    #[schemars(description = "Ending X coordinate")]
    pub end_x: i32,

    #[schemars(description = "Ending Y coordinate")]
    pub end_y: i32,

    #[schemars(description = "Swipe duration in milliseconds (optional)")]
    #[serde(default)]
    pub duration_ms: Option<u32>,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct InputTextParams {
    #[schemars(description = "Text to type on the device (requires focus on a text field)")]
    pub text: String,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct KeyeventParams {
    #[schemars(description = "Key to send (e.g., BACK, HOME, ENTER, or numeric keycode)")]
    pub key: String,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ShellParams {
    #[schemars(description = "Shell command to execute on the device")]
    pub command: String,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UiDumpParams {
    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindElementParams {
    #[schemars(description = "Text or content description to search for (substring match)")]
    #[serde(default)]
    pub text: Option<String>,

    #[schemars(description = "Resource ID to search for (substring match)")]
    #[serde(default)]
    pub resource_id: Option<String>,

    #[schemars(description = "Class name to filter by (substring match)")]
    #[serde(default)]
    pub class: Option<String>,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TapElementParams {
    #[schemars(description = "Text or content description to search for (substring match)")]
    #[serde(default)]
    pub text: Option<String>,

    #[schemars(description = "Resource ID to search for (substring match)")]
    #[serde(default)]
    pub resource_id: Option<String>,

    #[schemars(description = "Class name to filter by (substring match)")]
    #[serde(default)]
    pub class: Option<String>,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetActivityParams {
    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WaitForActivityParams {
    #[schemars(description = "Activity name or pattern to wait for")]
    pub activity: String,

    #[schemars(description = "Timeout in milliseconds (default: 10000)")]
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    #[schemars(description = "Device serial number (optional, auto-selects if only one device)")]
    #[serde(default)]
    pub device: Option<String>,
}
