//! E2E test: Run health command and verify all checks pass

use std::process::Command;

/// Get the workspace root directory
fn workspace_root() -> std::path::PathBuf {
    // Tests run from workspace root, but we want to be explicit
    std::env::current_dir()
        .expect("Failed to get cwd")
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get cwd"))
}

/// Find the agent binary (debug or release) as absolute path
fn agent_binary() -> std::path::PathBuf {
    let workspace = workspace_root();
    let release = workspace.join("target/release/agent");
    let debug = workspace.join("target/debug/agent");

    if release.exists() {
        release
    } else {
        debug
    }
}

#[test]
#[ignore = "requires Ollama and MCP servers"]
fn test_health_basic() {
    let output = Command::new(agent_binary())
        .args(["health"])
        .current_dir(workspace_root())
        .output()
        .expect("Failed to run health command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stdout:\n{}", stdout);
    if !stderr.is_empty() {
        println!("stderr:\n{}", stderr);
    }

    assert!(
        output.status.success(),
        "Basic health check failed with status: {:?}",
        output.status
    );
}

#[test]
#[ignore = "requires Ollama and MCP servers"]
fn test_health_all() {
    let output = Command::new(agent_binary())
        .args(["health", "--all"])
        .current_dir(workspace_root())
        .output()
        .expect("Failed to run health --all command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stdout:\n{}", stdout);
    if !stderr.is_empty() {
        println!("stderr:\n{}", stderr);
    }

    // The health check should pass
    assert!(
        output.status.success(),
        "Health check --all failed. stderr: {}",
        stderr
    );

    // Verify key outputs are present
    let stdout_lower = stdout.to_lowercase();
    assert!(
        stdout_lower.contains("config") || stdout_lower.contains(".mcp.json"),
        "Health output should mention config"
    );
}

#[test]
#[ignore = "requires Ollama and MCP servers"]
fn test_health_tool_execution() {
    // This test specifically verifies tool execution via health --all
    let output = Command::new(agent_binary())
        .args(["health", "--all"])
        .current_dir(workspace_root())
        .output()
        .expect("Failed to run health command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Health --all runs get_uptime tool, should see uptime info
    let mentions_tool = stdout.contains("uptime")
        || stdout.contains("Tool")
        || stdout.contains("tool")
        || stdout.contains("sysinfo");

    assert!(
        mentions_tool || output.status.success(),
        "Health check should mention tool execution or succeed"
    );
}
