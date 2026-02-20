use anyhow::{Context, Result};
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::validation::{strip_text_prefix, validate_png, PngError, PngInfo};

/// Screenshot capture result
#[derive(Debug)]
pub struct ScreenshotResult {
    pub data: Vec<u8>,
    pub info: PngInfo,
}

/// Capture a screenshot with validation and fallback strategies
pub async fn capture_screenshot(device: &str) -> Result<ScreenshotResult> {
    info!("Capturing screenshot from device {}", device);

    // Strategy 1: Direct exec-out (fastest)
    match capture_direct(device).await {
        Ok(result) => return Ok(result),
        Err(e) => {
            warn!("Direct capture failed: {}, trying fallback", e);
        }
    }

    // Strategy 2: Capture to device storage, then pull (more reliable)
    match capture_via_storage(device).await {
        Ok(result) => return Ok(result),
        Err(e) => {
            warn!("Storage capture failed: {}", e);
        }
    }

    anyhow::bail!("All screenshot capture strategies failed")
}

/// Direct capture using exec-out (fast but can fail on some devices)
async fn capture_direct(device: &str) -> Result<ScreenshotResult> {
    debug!("Attempting direct capture via exec-out");

    let output = Command::new("adb")
        .args(["-s", device, "exec-out", "screencap", "-p"])
        .output()
        .await
        .context("Failed to run screencap")?;

    if !output.status.success() {
        anyhow::bail!(
            "screencap failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let data = output.stdout;

    match validate_png(&data) {
        Ok(info) => {
            debug!("Direct capture successful: {}x{}", info.width, info.height);
            Ok(ScreenshotResult { data, info })
        }
        Err(PngError::TextPrefix(text)) => {
            warn!("PNG has text prefix: {}", text);
            if let Some(stripped) = strip_text_prefix(&data) {
                let stripped_data = stripped.to_vec();
                let info = validate_png(&stripped_data)
                    .context("PNG invalid even after stripping text prefix")?;
                Ok(ScreenshotResult {
                    data: stripped_data,
                    info,
                })
            } else {
                anyhow::bail!("Direct capture produced invalid PNG")
            }
        }
        Err(e) => {
            debug!("Direct capture validation failed: {}", e);
            Err(e.into())
        }
    }
}

/// Capture via device storage (slower but more reliable)
async fn capture_via_storage(device: &str) -> Result<ScreenshotResult> {
    debug!("Attempting capture via device storage");

    let remote_path = "/sdcard/adb_mcp_screenshot.png";

    // Capture to file on device
    let output = Command::new("adb")
        .args(["-s", device, "shell", "screencap", "-p", remote_path])
        .output()
        .await
        .context("Failed to capture to storage")?;

    if !output.status.success() {
        anyhow::bail!(
            "screencap to storage failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Pull the file using cat (binary-safe via exec-out)
    let data = Command::new("adb")
        .args(["-s", device, "exec-out", "cat", remote_path])
        .output()
        .await
        .context("Failed to pull screenshot")?
        .stdout;

    // Cleanup
    let _ = Command::new("adb")
        .args(["-s", device, "shell", "rm", "-f", remote_path])
        .output()
        .await;

    // Validate
    let info = validate_png(&data).context("Screenshot from storage is invalid")?;

    debug!(
        "Storage capture successful: {}x{}, {} bytes",
        info.width, info.height, info.size
    );

    Ok(ScreenshotResult { data, info })
}

/// Wake the device screen before capturing
pub async fn wake_device(device: &str) -> Result<()> {
    debug!("Waking device screen");

    Command::new("adb")
        .args(["-s", device, "shell", "input", "keyevent", "KEYCODE_WAKEUP"])
        .output()
        .await
        .context("Failed to wake device")?;

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    Ok(())
}
