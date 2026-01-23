//! MCP logs command handler

use anyhow::Result;

use crate::mcps::default_log_dir;

/// Handle the `mcps logs` command
pub async fn run_mcps_logs(lines: usize) -> Result<()> {
    let log_dir = default_log_dir();
    let log_file = log_dir.join("daemon.log");
    let err_file = log_dir.join("daemon.err");

    if !log_file.exists() && !err_file.exists() {
        println!("No daemon logs found.");
        println!("Expected location: {:?}", log_dir);
        return Ok(());
    }

    // Read stdout log
    if log_file.exists() {
        println!("=== Daemon stdout ({:?}) ===\n", log_file);
        let content = tokio::fs::read_to_string(&log_file).await?;
        let log_lines: Vec<&str> = content.lines().collect();

        let display_lines = if lines == 0 {
            &log_lines[..]
        } else {
            let start = log_lines.len().saturating_sub(lines);
            &log_lines[start..]
        };

        for line in display_lines {
            println!("{}", line);
        }
    }

    // Read stderr log
    if err_file.exists() {
        let err_content = tokio::fs::read_to_string(&err_file).await?;
        if !err_content.trim().is_empty() {
            println!("\n=== Daemon stderr ({:?}) ===\n", err_file);
            let err_lines: Vec<&str> = err_content.lines().collect();

            let display_lines = if lines == 0 {
                &err_lines[..]
            } else {
                let start = err_lines.len().saturating_sub(lines);
                &err_lines[start..]
            };

            for line in display_lines {
                println!("{}", line);
            }
        }
    }

    Ok(())
}
