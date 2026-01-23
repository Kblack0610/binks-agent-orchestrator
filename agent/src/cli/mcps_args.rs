//! MCP daemon subcommands
//!
//! Commands for managing the MCP server daemon.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum McpsCommands {
    /// Show MCP server status and tool cache
    Status {
        /// Show detailed tool list for each server
        #[arg(long, short)]
        verbose: bool,
    },
    /// Clear the tools cache and reconnect to servers
    Refresh,
    /// Start the MCP daemon (background supervisor for MCP servers)
    Start {
        /// Run as a background daemon process
        #[arg(long, short)]
        daemon: bool,
    },
    /// Stop the MCP daemon
    Stop,
    /// View daemon logs
    Logs {
        /// Number of lines to show (0 = all)
        #[arg(long, short, default_value = "50")]
        lines: usize,
    },
}
