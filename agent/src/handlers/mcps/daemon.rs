//! MCP daemon start/stop command handlers

use anyhow::Result;

use crate::config::McpConfig;
use crate::mcps::{
    default_log_dir, default_pid_path, default_socket_path, is_daemon_running, DaemonClient,
    McpDaemon,
};

/// Handle the `mcps start` command
pub async fn run_mcps_start(daemon: bool) -> Result<()> {
    // Check if daemon is already running
    if is_daemon_running().await {
        println!("MCP daemon is already running.");
        println!("Socket: {:?}", default_socket_path());
        return Ok(());
    }

    // Load MCP config
    let config = McpConfig::load()?.ok_or_else(|| anyhow::anyhow!("No .mcp.json found"))?;

    let socket_path = default_socket_path();
    let pid_path = default_pid_path();
    let log_dir = default_log_dir();

    // Ensure directories exist
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::create_dir_all(&log_dir).await?;

    if daemon {
        // Fork to background using std::process::Command
        println!("Starting MCP daemon in background...");

        let log_file = log_dir.join("daemon.log");
        let current_exe = std::env::current_exe()?;
        let current_dir = std::env::current_dir()?;

        // Re-execute ourselves with mcps start (without --daemon)
        let child = std::process::Command::new(&current_exe)
            .arg("mcps")
            .arg("start")
            .current_dir(&current_dir)
            .stdout(std::fs::File::create(&log_file)?)
            .stderr(std::fs::File::create(log_dir.join("daemon.err"))?)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn daemon: {}", e))?;

        // Write PID file
        let pid = child.id();
        tokio::fs::write(&pid_path, pid.to_string()).await?;

        // Wait briefly and check if daemon started
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        if is_daemon_running().await {
            println!("MCP daemon started successfully.");
            println!("  PID: {}", pid);
            println!("  Socket: {:?}", socket_path);
            println!("  Log: {:?}", log_file);
        } else {
            println!("Warning: Daemon may not have started. Check logs:");
            println!("  {:?}", log_file);
        }
    } else {
        // Run in foreground
        println!("Starting MCP daemon (foreground)...");
        println!("  Socket: {:?}", socket_path);
        println!("  Press Ctrl+C to stop.\n");

        // Write PID file for foreground mode too
        let pid = std::process::id();
        tokio::fs::write(&pid_path, pid.to_string()).await?;

        let mcp_daemon = McpDaemon::new(config, socket_path);
        mcp_daemon.run().await?;

        // Cleanup PID file
        let _ = tokio::fs::remove_file(&pid_path).await;
    }

    Ok(())
}

/// Handle the `mcps stop` command
pub async fn run_mcps_stop() -> Result<()> {
    let socket_path = default_socket_path();
    let pid_path = default_pid_path();

    // First try to send shutdown command via socket
    if is_daemon_running().await {
        println!("Sending shutdown command to daemon...");
        let client = DaemonClient::with_socket_path(socket_path.clone());
        match client.shutdown().await {
            Ok(_) => {
                println!("Daemon shutdown initiated.");
            }
            Err(e) => {
                println!("Warning: shutdown command failed: {}", e);
            }
        }

        // Wait a moment for clean shutdown
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Check if PID file exists and kill process if needed
    if pid_path.exists() {
        let pid_str = tokio::fs::read_to_string(&pid_path).await?;
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            // Check if process is still running
            #[cfg(unix)]
            {
                // Try to send SIGTERM
                let _ = unsafe { libc::kill(pid, libc::SIGTERM) };
            }
        }

        // Remove PID file
        let _ = tokio::fs::remove_file(&pid_path).await;
    }

    // Remove socket file if it exists
    let _ = tokio::fs::remove_file(&socket_path).await;

    println!("MCP daemon stopped.");
    Ok(())
}
