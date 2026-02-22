mod device;
mod input;
mod screenshot;
mod shell;
mod ui;

pub use device::*;
pub use input::*;
pub use screenshot::*;
pub use shell::*;
pub use ui::*;

use std::process::Output;
use std::time::Duration;
use tokio::process::Command;

/// Default timeout for most ADB commands (30 seconds).
pub const ADB_TIMEOUT: Duration = Duration::from_secs(30);

/// Shorter timeout for cleanup operations (5 seconds).
pub const ADB_CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);

/// Run an ADB command with a timeout, preventing indefinite hangs.
pub async fn run_adb_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> anyhow::Result<Output> {
    tokio::time::timeout(timeout, command.output())
        .await
        .map_err(|_| anyhow::anyhow!("ADB command timed out after {timeout:?}"))?
        .map_err(|e| anyhow::anyhow!("ADB command failed: {e}"))
}
