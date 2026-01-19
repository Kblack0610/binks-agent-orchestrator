//! MCP Supervisor Daemon
//!
//! This module provides a background daemon for managing MCP server lifecycles.
//! The daemon solves the Send/Sync problem by keeping all non-Send MCP connections
//! in a single process and communicating with agents via Unix socket.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     Unix Socket     ┌──────────────────┐
//! │   Agent     │ ←────────────────→  │   MCP Daemon     │
//! └─────────────┘                     └────────┬─────────┘
//!                                              │
//!                   ┌──────────────────────────┼──────────────────────────┐
//!                   ↓                          ↓                          ↓
//!           ┌───────────────┐        ┌───────────────┐        ┌───────────────┐
//!           │  github-gh    │        │  kubernetes   │        │   sysinfo     │
//!           │  (persistent) │        │  (persistent) │        │  (persistent) │
//!           └───────────────┘        └───────────────┘        └───────────────┘
//! ```
//!
//! # Usage
//!
//! Start the daemon:
//! ```bash
//! agent mcps start --daemon
//! ```
//!
//! Check status:
//! ```bash
//! agent mcps status
//! ```
//!
//! Stop the daemon:
//! ```bash
//! agent mcps stop
//! ```

mod client;
mod daemon;
mod protocol;

pub use client::DaemonClient;
pub use daemon::{is_daemon_running, ping_daemon, McpDaemon};
pub use protocol::*;
