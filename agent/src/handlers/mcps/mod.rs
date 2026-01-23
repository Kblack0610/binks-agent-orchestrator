//! MCP server management handlers
//!
//! Subcommands for managing MCP daemon and connections.

mod daemon;
mod logs;
mod refresh;
mod status;

use anyhow::Result;

use crate::cli::McpsCommands;

pub use daemon::{run_mcps_start, run_mcps_stop};
pub use logs::run_mcps_logs;
pub use refresh::run_mcps_refresh;
pub use status::run_mcps_status;

/// Dispatch mcps subcommands to their handlers
pub async fn run_mcps_command(command: McpsCommands) -> Result<()> {
    match command {
        McpsCommands::Status { verbose } => run_mcps_status(verbose).await,
        McpsCommands::Refresh => run_mcps_refresh().await,
        McpsCommands::Start { daemon } => run_mcps_start(daemon).await,
        McpsCommands::Stop => run_mcps_stop().await,
        McpsCommands::Logs { lines } => run_mcps_logs(lines).await,
    }
}
