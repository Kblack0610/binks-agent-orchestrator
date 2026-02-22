use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use super::{run_adb_with_timeout, ADB_TIMEOUT};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub serial: String,
    pub state: String,
    pub model: Option<String>,
    pub product: Option<String>,
}

/// List all connected ADB devices
pub async fn list_devices() -> Result<Vec<Device>> {
    let output = run_adb_with_timeout(
        Command::new("adb").args(["devices", "-l"]),
        ADB_TIMEOUT,
    )
    .await?;

    if !output.status.success() {
        anyhow::bail!(
            "adb devices failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in stdout.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let serial = parts[0].to_string();
            let state = parts[1].to_string();

            let mut model = None;
            let mut product = None;

            for part in &parts[2..] {
                if let Some(val) = part.strip_prefix("model:") {
                    model = Some(val.replace('_', " "));
                } else if let Some(val) = part.strip_prefix("product:") {
                    product = Some(val.to_string());
                }
            }

            devices.push(Device {
                serial,
                state,
                model,
                product,
            });
        }
    }

    Ok(devices)
}

/// Get a single device, auto-selecting if only one is connected
pub async fn get_device(serial: Option<&str>) -> Result<String> {
    match serial {
        Some(s) => Ok(s.to_string()),
        None => {
            let devices = list_devices().await?;
            match devices.len() {
                0 => anyhow::bail!("No ADB devices connected"),
                1 => Ok(devices[0].serial.clone()),
                n => anyhow::bail!(
                    "Multiple devices connected ({}), please specify device serial: {:?}",
                    n,
                    devices.iter().map(|d| &d.serial).collect::<Vec<_>>()
                ),
            }
        }
    }
}
