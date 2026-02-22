use anyhow::Result;
use tokio::process::Command;

use super::{run_adb_with_timeout, ADB_TIMEOUT};

/// Tap at coordinates
pub async fn tap(device: &str, x: i32, y: i32) -> Result<()> {
    run_adb_with_timeout(
        Command::new("adb").args([
            "-s",
            device,
            "shell",
            "input",
            "tap",
            &x.to_string(),
            &y.to_string(),
        ]),
        ADB_TIMEOUT,
    )
    .await?;

    Ok(())
}

/// Swipe from one point to another
pub async fn swipe(
    device: &str,
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    duration_ms: Option<u32>,
) -> Result<()> {
    let mut args = vec![
        "-s".to_string(),
        device.to_string(),
        "shell".to_string(),
        "input".to_string(),
        "swipe".to_string(),
        start_x.to_string(),
        start_y.to_string(),
        end_x.to_string(),
        end_y.to_string(),
    ];

    if let Some(duration) = duration_ms {
        args.push(duration.to_string());
    }

    run_adb_with_timeout(Command::new("adb").args(&args), ADB_TIMEOUT).await?;

    Ok(())
}

/// Input text
pub async fn input_text(device: &str, text: &str) -> Result<()> {
    // Escape special characters for shell
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
        .replace(' ', "%s"); // ADB input text uses %s for space

    run_adb_with_timeout(
        Command::new("adb").args(["-s", device, "shell", "input", "text", &escaped]),
        ADB_TIMEOUT,
    )
    .await?;

    Ok(())
}

/// Send a key event
pub async fn keyevent(device: &str, key: &str) -> Result<()> {
    // Support both numeric keycodes and named keycodes
    let keycode = if key.starts_with("KEYCODE_") || key.parse::<i32>().is_ok() {
        key.to_string()
    } else {
        format!("KEYCODE_{}", key.to_uppercase())
    };

    run_adb_with_timeout(
        Command::new("adb").args(["-s", device, "shell", "input", "keyevent", &keycode]),
        ADB_TIMEOUT,
    )
    .await?;

    Ok(())
}
