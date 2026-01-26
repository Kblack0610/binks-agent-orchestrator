//! Verify E2E test prerequisites before running tests

use std::path::PathBuf;
use std::time::Duration;

/// Get the workspace root directory (contains target/ and Cargo.toml with [workspace])
pub fn workspace_root() -> PathBuf {
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

    // Fallback to cwd
    std::env::current_dir().expect("Failed to get cwd")
}

/// Default Ollama URL
pub fn ollama_url() -> String {
    std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string())
}

/// Check if Ollama is running and accessible
pub fn check_ollama() -> bool {
    let url = ollama_url();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build client");

    client
        .get(format!("{}/api/tags", url))
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Check if workspace binaries are built
pub fn check_binaries_built() -> bool {
    let workspace = workspace_root();
    let binaries = ["agent", "sysinfo-mcp", "github-gh-mcp", "inbox-mcp"];

    binaries.iter().all(|name| {
        let debug_path = workspace.join("target").join("debug").join(name);
        let release_path = workspace.join("target").join("release").join(name);
        debug_path.exists() || release_path.exists()
    })
}

/// Check if .mcp.json config exists
pub fn check_mcp_config() -> bool {
    let workspace = workspace_root();
    workspace.join(".mcp.json").exists()
}

#[test]
#[ignore = "prerequisites check - run first"]
fn test_prerequisites() {
    println!("\n=== E2E Prerequisites Check ===\n");

    let workspace = workspace_root();
    println!("Workspace root: {}", workspace.display());

    // Check Ollama
    let ollama_ok = check_ollama();
    println!(
        "Ollama ({}): {}",
        ollama_url(),
        if ollama_ok {
            "✓ Running"
        } else {
            "✗ Not accessible"
        }
    );

    // Check binaries
    let binaries_ok = check_binaries_built();
    println!(
        "Workspace binaries: {}",
        if binaries_ok {
            "✓ Built"
        } else {
            "✗ Not found (run: cargo build --workspace)"
        }
    );

    // Check config
    let config_ok = check_mcp_config();
    println!(
        ".mcp.json config: {}",
        if config_ok {
            "✓ Found"
        } else {
            "✗ Not found"
        }
    );

    println!();

    // Fail if prerequisites not met
    assert!(ollama_ok, "Ollama not running. Start with: ollama serve");
    assert!(
        binaries_ok,
        "Binaries not built. Run: cargo build --workspace (workspace: {})",
        workspace.display()
    );
    assert!(
        config_ok,
        ".mcp.json not found in workspace root: {}",
        workspace.display()
    );

    println!("=== All prerequisites met ===\n");
}
