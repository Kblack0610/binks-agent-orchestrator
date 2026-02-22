use anyhow::Result;
use tokio::process::Command;

use super::{run_adb_with_timeout, ADB_TIMEOUT};

/// Execute a shell command on the device
pub async fn shell(device: &str, command: &str) -> Result<ShellOutput> {
    let output =
        run_adb_with_timeout(Command::new("adb").args(["-s", device, "shell", command]), ADB_TIMEOUT)
            .await?;

    Ok(ShellOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// Execute exec-out command (binary-safe output)
pub async fn exec_out(device: &str, command: &str) -> Result<Vec<u8>> {
    let args: Vec<&str> = command.split_whitespace().collect();

    let output = run_adb_with_timeout(
        Command::new("adb")
            .args(["-s", device, "exec-out"])
            .args(&args),
        ADB_TIMEOUT,
    )
    .await?;

    if !output.status.success() {
        anyhow::bail!(
            "exec-out failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(output.stdout)
}

#[derive(Debug, Clone)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
