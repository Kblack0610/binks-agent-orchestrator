//! Verify E2E test prerequisites before running tests

use std::path::Path;
use std::time::Duration;

/// Default Ollama URL
pub fn ollama_url() -> String {
    std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string())
}

/// Default model to use
pub fn ollama_model() -> String {
    std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.1:8b".to_string())
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
    let binaries = ["agent", "sysinfo-mcp", "github-gh-mcp", "inbox-mcp"];

    binaries.iter().all(|name| {
        let debug_path = format!("../target/debug/{}", name);
        let release_path = format!("../target/release/{}", name);
        Path::new(&debug_path).exists() || Path::new(&release_path).exists()
    })
}

/// Check if .mcp.json config exists
pub fn check_mcp_config() -> bool {
    // Check both workspace root and agent directory
    Path::new("../.mcp.json").exists() || Path::new(".mcp.json").exists()
}

#[test]
#[ignore = "prerequisites check - run first"]
fn test_prerequisites() {
    println!("\n=== E2E Prerequisites Check ===\n");

    // Check Ollama
    let ollama_ok = check_ollama();
    println!(
        "Ollama ({}): {}",
        ollama_url(),
        if ollama_ok { "✓ Running" } else { "✗ Not accessible" }
    );

    // Check binaries
    let binaries_ok = check_binaries_built();
    println!(
        "Workspace binaries: {}",
        if binaries_ok { "✓ Built" } else { "✗ Not found (run: cargo build --workspace)" }
    );

    // Check config
    let config_ok = check_mcp_config();
    println!(
        ".mcp.json config: {}",
        if config_ok { "✓ Found" } else { "✗ Not found" }
    );

    println!();

    // Fail if prerequisites not met
    assert!(ollama_ok, "Ollama not running. Start with: ollama serve");
    assert!(binaries_ok, "Binaries not built. Run: cargo build --workspace");
    assert!(config_ok, ".mcp.json not found in workspace root");

    println!("=== All prerequisites met ===\n");
}
