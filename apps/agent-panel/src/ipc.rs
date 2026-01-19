//! IPC for toggle commands
//!
//! Uses a Unix socket for external control (e.g., from Hyprland keybind)

use std::path::PathBuf;

/// Get the IPC socket path
pub fn socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(runtime_dir).join("agent-panel.sock")
}

// TODO: Implement IPC server in subscription
// For now, visibility is controlled via auto-popup on urgent agents
