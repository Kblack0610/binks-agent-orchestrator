//! E2E tests for MCP tier-based server filtering
//!
//! Tests the integration between config loading, tier parsing, and server filtering
//! based on model size.

use binks_agent::config::McpConfig;
use binks_agent::mcp::model_size::{parse_model_size, ModelSize};
use binks_agent::mcp::McpClientPool;
use std::path::PathBuf;

/// Get the workspace root directory (contains target/ and Cargo.toml with [workspace])
fn workspace_root() -> PathBuf {
    let mut current = std::env::current_dir().expect("Failed to get cwd");

    loop {
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

#[test]
fn test_tier_assignments_in_config() {
    let workspace = workspace_root();
    let config_path = workspace.join(".mcp.json");

    let config = McpConfig::load_from_path(&config_path).expect("Failed to load .mcp.json");

    // Verify tier assignments match expectations
    let servers = &config.mcp_servers;

    // Tier 1 - Essential
    assert_eq!(
        servers.get("filesystem").map(|s| s.tier),
        Some(1),
        "filesystem should be tier 1"
    );
    assert_eq!(
        servers.get("sysinfo").map(|s| s.tier),
        Some(1),
        "sysinfo should be tier 1"
    );

    // Tier 2 - Standard
    assert_eq!(
        servers.get("github-gh").map(|s| s.tier),
        Some(2),
        "github-gh should be tier 2"
    );
    assert_eq!(
        servers.get("inbox").map(|s| s.tier),
        Some(2),
        "inbox should be tier 2"
    );
    assert_eq!(
        servers.get("notify").map(|s| s.tier),
        Some(2),
        "notify should be tier 2"
    );

    // Tier 3 - Extended
    assert_eq!(
        servers.get("kubernetes").map(|s| s.tier),
        Some(3),
        "kubernetes should be tier 3"
    );
    assert_eq!(
        servers.get("ssh").map(|s| s.tier),
        Some(3),
        "ssh should be tier 3"
    );
    assert_eq!(
        servers.get("web-search").map(|s| s.tier),
        Some(3),
        "web-search should be tier 3"
    );

    // Tier 4 - Agent only
    assert_eq!(
        servers.get("agent").map(|s| s.tier),
        Some(4),
        "agent should be tier 4"
    );
}

#[tokio::test]
#[ignore = "requires MCP servers running"]
async fn test_server_filtering_by_tier() {
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

    // Test tier 1 filtering (Essential only)
    let tier1_servers = pool.server_names_for_tier(1);
    println!("Tier 1 servers: {:?}", tier1_servers);
    assert!(tier1_servers.contains(&"filesystem".to_string()));
    assert!(tier1_servers.contains(&"sysinfo".to_string()));
    assert!(!tier1_servers.contains(&"github-gh".to_string()));
    assert!(!tier1_servers.contains(&"kubernetes".to_string()));
    assert_eq!(
        tier1_servers.len(),
        2,
        "Tier 1 should have exactly 2 servers"
    );

    // Test tier 2 filtering (Essential + Standard)
    let tier2_servers = pool.server_names_for_tier(2);
    println!("Tier 2 servers: {:?}", tier2_servers);
    assert!(tier2_servers.contains(&"filesystem".to_string()));
    assert!(tier2_servers.contains(&"sysinfo".to_string()));
    assert!(tier2_servers.contains(&"github-gh".to_string()));
    assert!(tier2_servers.contains(&"inbox".to_string()));
    assert!(tier2_servers.contains(&"notify".to_string()));
    assert!(!tier2_servers.contains(&"kubernetes".to_string()));
    assert_eq!(
        tier2_servers.len(),
        5,
        "Tier 2 should have exactly 5 servers"
    );

    // Test tier 3 filtering (All except agent-only)
    let tier3_servers = pool.server_names_for_tier(3);
    println!("Tier 3 servers: {:?}", tier3_servers);
    assert!(tier3_servers.contains(&"kubernetes".to_string()));
    assert!(tier3_servers.contains(&"ssh".to_string()));
    assert!(tier3_servers.contains(&"web-search".to_string()));
    assert!(!tier3_servers.contains(&"agent".to_string()));
    assert_eq!(
        tier3_servers.len(),
        8,
        "Tier 3 should have exactly 8 servers"
    );

    // Test tier 4 filtering (All servers)
    // Note: The "agent" server (tier 4) is the agent binary itself in serve mode.
    // It may not be loadable in test context (would be recursive), so we just verify
    // that tier 4 includes all tier 3 servers.
    let tier4_servers = pool.server_names_for_tier(4);
    println!("Tier 4 servers: {:?}", tier4_servers);
    // Tier 4 should include everything tier 3 has
    for server in &tier3_servers {
        assert!(
            tier4_servers.contains(server),
            "Tier 4 should include tier 3 server: {}",
            server
        );
    }
    // Should have at least 8 servers (tier 3 servers). Agent may or may not be present.
    assert!(
        tier4_servers.len() >= 8,
        "Tier 4 should have at least 8 servers, got {}",
        tier4_servers.len()
    );

    drop(pool);
}

#[tokio::test]
#[ignore = "requires MCP servers running"]
async fn test_server_filtering_by_model_size() {
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

    // Small model -> Tier 1 only
    let small_size = parse_model_size("llama3.1:8b");
    assert_eq!(small_size, ModelSize::Small);
    let small_servers = pool.server_names_for_model_size(small_size);
    println!("Small model servers: {:?}", small_servers);
    assert_eq!(
        small_servers.len(),
        2,
        "Small models should only get tier 1 servers"
    );

    // Medium model -> Tier 1+2
    let medium_size = parse_model_size("qwen3-coder:30b");
    assert_eq!(medium_size, ModelSize::Medium);
    let medium_servers = pool.server_names_for_model_size(medium_size);
    println!("Medium model servers: {:?}", medium_servers);
    assert_eq!(
        medium_servers.len(),
        5,
        "Medium models should get tier 1-2 servers"
    );

    // Large model -> Tier 1+2+3
    let large_size = parse_model_size("llama3.1:70b");
    assert_eq!(large_size, ModelSize::Large);
    let large_servers = pool.server_names_for_model_size(large_size);
    println!("Large model servers: {:?}", large_servers);
    assert_eq!(
        large_servers.len(),
        8,
        "Large models should get tier 1-3 servers"
    );

    // Unknown model -> Conservative tier 1
    let unknown_size = parse_model_size("gpt-4");
    assert_eq!(unknown_size, ModelSize::Unknown);
    let unknown_servers = pool.server_names_for_model_size(unknown_size);
    println!("Unknown model servers: {:?}", unknown_servers);
    assert_eq!(
        unknown_servers.len(),
        2,
        "Unknown models should conservatively get tier 1 only"
    );

    drop(pool);
}

#[test]
fn test_model_size_tier_mapping() {
    // Verify the mapping from model size to default tier
    assert_eq!(ModelSize::Small.default_max_tier(), 1);
    assert_eq!(ModelSize::Medium.default_max_tier(), 2);
    assert_eq!(ModelSize::Large.default_max_tier(), 3);
    assert_eq!(ModelSize::Unknown.default_max_tier(), 1);
}

#[test]
fn test_model_size_parsing_integration() {
    // Test various model formats work correctly
    let test_cases = vec![
        ("llama3.1:8b", ModelSize::Small),
        ("llama3.1:8b-q4_0", ModelSize::Small),
        ("mistral:7b-instruct", ModelSize::Small),
        ("phi4:14b", ModelSize::Medium),
        ("qwen3-coder:30b", ModelSize::Medium),
        ("deepseek-r1:70b", ModelSize::Large),
        ("deepseek-r1:671b", ModelSize::Large),
        ("gpt-4o", ModelSize::Unknown),
        ("claude-3-opus", ModelSize::Unknown),
    ];

    for (model, expected_size) in test_cases {
        let actual = parse_model_size(model);
        assert_eq!(
            actual, expected_size,
            "Model '{}' expected {:?}, got {:?}",
            model, expected_size, actual
        );
    }
}
