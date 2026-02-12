//! E2E test: Direct tool execution via MCP client

use binks_agent::mcp::McpClientPool;
use std::path::PathBuf;

/// Get the workspace root directory (contains target/ and Cargo.toml with [workspace])
fn workspace_root() -> PathBuf {
    let mut current = std::env::current_dir().expect("Failed to get cwd");

    loop {
        // Workspace root has target/ directory and a Cargo.toml
        // This distinguishes from crate subdirectories like agent/
        let has_target = current.join("target").is_dir();
        let has_cargo = current.join("Cargo.toml").exists();
        let has_agent_subdir = current.join("agent").is_dir();

        if has_target && has_cargo && has_agent_subdir {
            return current;
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    std::env::current_dir().expect("Failed to get cwd")
}

#[tokio::test]
#[ignore = "requires MCP servers running"]
async fn test_mcp_pool_loads() {
    // Change to workspace root for config loading
    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let workspace = workspace_root();
    println!("Using workspace root: {}", workspace.display());
    std::env::set_current_dir(&workspace).expect("Failed to change to workspace root");

    let result = McpClientPool::load();

    // Restore directory
    std::env::set_current_dir(original_dir).expect("Failed to restore dir");

    let pool = result.expect("Failed to load MCP config");
    assert!(
        pool.is_some(),
        "Expected .mcp.json to be found at: {}",
        workspace.join(".mcp.json").display()
    );

    let pool = pool.unwrap();
    println!("Loaded MCP pool with servers");

    // Cleanup
    drop(pool);
}

#[tokio::test]
#[ignore = "requires MCP servers running"]
async fn test_list_tools_from_sysinfo() {
    // Change to workspace root
    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let workspace = workspace_root();
    std::env::set_current_dir(&workspace).expect("Failed to change to workspace root");

    let pool = McpClientPool::load()
        .expect("Failed to load MCP config")
        .unwrap_or_else(|| {
            panic!(
                "No .mcp.json found at: {}",
                workspace.join(".mcp.json").display()
            )
        });

    // Restore directory
    std::env::set_current_dir(&original_dir).expect("Failed to restore dir");

    let mut pool = pool;

    let tools = pool
        .list_tools_from("sysinfo")
        .await
        .expect("Failed to list sysinfo tools");

    println!("Found {} tools from sysinfo:", tools.len());
    for tool in &tools {
        println!("  - {}", tool.name);
    }

    // Sysinfo should have multiple tools
    assert!(!tools.is_empty(), "No tools found from sysinfo");

    let tool_names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();

    // These tools should exist in sysinfo-mcp
    assert!(
        tool_names.contains(&"get_uptime"),
        "Missing get_uptime tool. Found: {:?}",
        tool_names
    );

    drop(pool);
}

#[tokio::test]
#[ignore = "requires MCP servers running"]
async fn test_call_get_uptime_tool() {
    // Change to workspace root
    let original_dir = std::env::current_dir().expect("Failed to get cwd");
    let workspace = workspace_root();
    std::env::set_current_dir(&workspace).expect("Failed to change to workspace root");

    let pool = McpClientPool::load()
        .expect("Failed to load MCP config")
        .unwrap_or_else(|| {
            panic!(
                "No .mcp.json found at: {}",
                workspace.join(".mcp.json").display()
            )
        });

    std::env::set_current_dir(&original_dir).expect("Failed to restore dir");

    let mut pool = pool;

    let result = pool
        .call_tool("get_uptime", None)
        .await
        .expect("Failed to call get_uptime");

    println!("Tool result: {:?}", result);

    // Verify we got content back
    assert!(!result.content.is_empty(), "No content returned from tool");

    // Extract text content
    let text_content: Vec<_> = result
        .content
        .iter()
        .filter_map(|c| match &c.raw {
            rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();

    assert!(!text_content.is_empty(), "No text content in tool response");

    let response_text = text_content.join(" ");
    println!("Response text: {}", response_text);

    // Should contain uptime-related info
    let has_uptime_info = response_text.contains("uptime")
        || response_text.contains("seconds")
        || response_text.contains("boot")
        || response_text.contains("\"seconds\""); // JSON field

    assert!(
        has_uptime_info,
        "Response doesn't contain uptime info: {}",
        response_text
    );

    drop(pool);
}
