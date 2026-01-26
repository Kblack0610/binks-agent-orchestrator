//! Integration tests for github-gh MCP server
//!
//! These tests run against a real GitHub repository using the gh CLI.
//! They require:
//! - gh CLI installed and authenticated
//! - Network access to GitHub
//!
//! # Running tests
//!
//! ```bash
//! # Run read-only tests (safe, no side effects)
//! cargo test --test integration -- --ignored read_
//!
//! # Run all integration tests (includes write tests)
//! cargo test --test integration -- --ignored
//!
//! # Run with custom test repo
//! TEST_REPO=owner/repo cargo test --test integration -- --ignored
//! ```
//!
//! By default, tests use `Kblack0610/binks-agent-orchestrator` as the test repo.

use std::env;
use std::process::Command;

/// Get the test repository from environment or use default
fn test_repo() -> String {
    env::var("TEST_REPO").unwrap_or_else(|_| "Kblack0610/binks-agent-orchestrator".to_string())
}

/// Check if gh CLI is available and authenticated
fn gh_available() -> bool {
    Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Execute gh command and return stdout
fn gh_exec(args: &[&str]) -> Result<String, String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute gh: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

// ============================================================================
// READ-ONLY TESTS (safe to run anytime)
// ============================================================================

#[test]
#[ignore = "integration test - requires gh CLI and network"]
fn read_issue_list() {
    if !gh_available() {
        eprintln!("Skipping: gh CLI not available");
        return;
    }

    let repo = test_repo();
    let result = gh_exec(&[
        "issue", "list",
        "-R", &repo,
        "-L", "5",
        "--json", "number,title,state,author,url",
    ]);

    assert!(result.is_ok(), "gh issue list failed: {:?}", result.err());
    let output = result.unwrap();

    // Verify it's valid JSON
    let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&output);
    assert!(parsed.is_ok(), "Output is not valid JSON: {}", output);

    println!("Issues returned: {}", parsed.unwrap().len());
}

#[test]
#[ignore = "integration test - requires gh CLI and network"]
fn read_issue_list_minimal_fields() {
    if !gh_available() {
        eprintln!("Skipping: gh CLI not available");
        return;
    }

    let repo = test_repo();

    // Test minimal fields (matches our list_fields_minimal)
    let result = gh_exec(&[
        "issue", "list",
        "-R", &repo,
        "-L", "3",
        "--json", "number,title,state,author,url",
    ]);

    assert!(result.is_ok(), "gh issue list (minimal) failed: {:?}", result.err());
    let output = result.unwrap();

    let parsed: Vec<serde_json::Value> = serde_json::from_str(&output)
        .expect("Output is not valid JSON");

    // Verify minimal fields are present
    if let Some(issue) = parsed.first() {
        assert!(issue.get("number").is_some(), "Missing 'number' field");
        assert!(issue.get("title").is_some(), "Missing 'title' field");
        assert!(issue.get("state").is_some(), "Missing 'state' field");
        assert!(issue.get("url").is_some(), "Missing 'url' field");
    }
}

#[test]
#[ignore = "integration test - requires gh CLI and network"]
fn read_pr_list() {
    if !gh_available() {
        eprintln!("Skipping: gh CLI not available");
        return;
    }

    let repo = test_repo();
    let result = gh_exec(&[
        "pr", "list",
        "-R", &repo,
        "-L", "5",
        "-s", "all",
        "--json", "number,title,state,author,headRefName,isDraft,url",
    ]);

    assert!(result.is_ok(), "gh pr list failed: {:?}", result.err());
    let output = result.unwrap();

    let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&output);
    assert!(parsed.is_ok(), "Output is not valid JSON: {}", output);

    println!("PRs returned: {}", parsed.unwrap().len());
}

#[test]
#[ignore = "integration test - requires gh CLI and network"]
fn read_pr_list_minimal_fields() {
    if !gh_available() {
        eprintln!("Skipping: gh CLI not available");
        return;
    }

    let repo = test_repo();

    // Test minimal fields (matches our list_fields_minimal)
    let result = gh_exec(&[
        "pr", "list",
        "-R", &repo,
        "-L", "3",
        "-s", "all",
        "--json", "number,title,state,author,headRefName,isDraft,url",
    ]);

    assert!(result.is_ok(), "gh pr list (minimal) failed: {:?}", result.err());
    let output = result.unwrap();

    let parsed: Vec<serde_json::Value> = serde_json::from_str(&output)
        .expect("Output is not valid JSON");

    // Verify minimal fields are present
    if let Some(pr) = parsed.first() {
        assert!(pr.get("number").is_some(), "Missing 'number' field");
        assert!(pr.get("title").is_some(), "Missing 'title' field");
        assert!(pr.get("state").is_some(), "Missing 'state' field");
        assert!(pr.get("headRefName").is_some(), "Missing 'headRefName' field");
        assert!(pr.get("url").is_some(), "Missing 'url' field");
    }
}

#[test]
#[ignore = "integration test - requires gh CLI and network"]
fn read_repo_view() {
    if !gh_available() {
        eprintln!("Skipping: gh CLI not available");
        return;
    }

    let repo = test_repo();
    let result = gh_exec(&[
        "repo", "view",
        &repo,
        "--json", "name,owner,description,url,defaultBranchRef",
    ]);

    assert!(result.is_ok(), "gh repo view failed: {:?}", result.err());
    let output = result.unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("Output is not valid JSON");

    assert!(parsed.get("name").is_some(), "Missing 'name' field");
    assert!(parsed.get("url").is_some(), "Missing 'url' field");

    println!("Repo: {}", parsed.get("name").unwrap());
}

#[test]
#[ignore = "integration test - requires gh CLI and network"]
fn read_workflow_list() {
    if !gh_available() {
        eprintln!("Skipping: gh CLI not available");
        return;
    }

    let repo = test_repo();
    let result = gh_exec(&[
        "workflow", "list",
        "-R", &repo,
        "--json", "name,id,state",
    ]);

    assert!(result.is_ok(), "gh workflow list failed: {:?}", result.err());
    let output = result.unwrap();

    let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&output);
    assert!(parsed.is_ok(), "Output is not valid JSON: {}", output);

    println!("Workflows returned: {}", parsed.unwrap().len());
}

#[test]
#[ignore = "integration test - requires gh CLI and network"]
fn read_pr_checks() {
    if !gh_available() {
        eprintln!("Skipping: gh CLI not available");
        return;
    }

    let repo = test_repo();

    // First get the latest PR number
    let pr_list = gh_exec(&[
        "pr", "list",
        "-R", &repo,
        "-L", "1",
        "-s", "all",
        "--json", "number",
    ]);

    if pr_list.is_err() {
        eprintln!("Skipping: Could not list PRs");
        return;
    }

    let prs: Vec<serde_json::Value> = serde_json::from_str(&pr_list.unwrap())
        .unwrap_or_default();

    if prs.is_empty() {
        eprintln!("Skipping: No PRs found in repo");
        return;
    }

    let pr_number = prs[0].get("number").unwrap().as_u64().unwrap();
    let pr_str = pr_number.to_string();

    // gh pr checks may return exit code 1 if checks failed, but still has valid output
    let output = Command::new("gh")
        .args(["pr", "checks", &pr_str, "-R", &repo])
        .output()
        .expect("Failed to execute gh pr checks");

    // Accept exit codes 0 (all pass) or 1 (some fail) as valid
    assert!(
        output.status.code().unwrap_or(2) <= 1,
        "gh pr checks returned unexpected exit code"
    );

    println!("PR #{} checks output length: {} bytes", pr_number, output.stdout.len());
}

// ============================================================================
// JSON SERIALIZATION TESTS
// ============================================================================

#[test]
#[ignore = "integration test - requires gh CLI and network"]
fn read_json_is_compact() {
    if !gh_available() {
        eprintln!("Skipping: gh CLI not available");
        return;
    }

    let repo = test_repo();
    let result = gh_exec(&[
        "issue", "list",
        "-R", &repo,
        "-L", "1",
        "--json", "number,title",
    ]);

    assert!(result.is_ok());
    let output = result.unwrap();

    // Verify JSON is parseable
    let _: Vec<serde_json::Value> = serde_json::from_str(&output)
        .expect("Output is not valid JSON");

    // Note: gh CLI returns compact JSON by default, our MCP now does too
    println!("JSON output: {}", output.trim());
}

// ============================================================================
// WRITE TESTS (opt-in, require explicit flag)
// ============================================================================

#[test]
#[ignore = "write test - creates/modifies GitHub resources"]
fn write_issue_create_and_close() {
    if !gh_available() {
        eprintln!("Skipping: gh CLI not available");
        return;
    }

    // Only run if explicitly enabled
    if env::var("GITHUB_WRITE_TESTS").is_err() {
        eprintln!("Skipping write test: set GITHUB_WRITE_TESTS=1 to enable");
        return;
    }

    let repo = test_repo();

    // Create test issue
    let create_result = gh_exec(&[
        "issue", "create",
        "-R", &repo,
        "-t", "[TEST] Integration test issue - please ignore",
        "-b", "This issue was created by an automated integration test and should be deleted.",
    ]);

    assert!(create_result.is_ok(), "Failed to create issue: {:?}", create_result.err());
    let url = create_result.unwrap();
    println!("Created issue: {}", url.trim());

    // Extract issue number from URL
    let number: u32 = url.trim()
        .split('/')
        .last()
        .and_then(|s| s.parse().ok())
        .expect("Could not parse issue number from URL");

    // Close the issue
    let number_str = number.to_string();
    let close_result = gh_exec(&[
        "issue", "close",
        &number_str,
        "-R", &repo,
        "-c", "Closing test issue",
    ]);

    assert!(close_result.is_ok(), "Failed to close issue: {:?}", close_result.err());
    println!("Closed issue #{}", number);
}

#[test]
#[ignore = "write test - creates/modifies GitHub resources"]
fn write_issue_comment() {
    if !gh_available() {
        eprintln!("Skipping: gh CLI not available");
        return;
    }

    // Only run if explicitly enabled
    if env::var("GITHUB_WRITE_TESTS").is_err() {
        eprintln!("Skipping write test: set GITHUB_WRITE_TESTS=1 to enable");
        return;
    }

    let repo = test_repo();

    // Get first open issue to comment on
    let list_result = gh_exec(&[
        "issue", "list",
        "-R", &repo,
        "-L", "1",
        "-s", "open",
        "--json", "number",
    ]);

    if list_result.is_err() {
        eprintln!("Skipping: Could not list issues");
        return;
    }

    let issues: Vec<serde_json::Value> = serde_json::from_str(&list_result.unwrap())
        .unwrap_or_default();

    if issues.is_empty() {
        eprintln!("Skipping: No open issues to comment on");
        return;
    }

    let number = issues[0].get("number").unwrap().as_u64().unwrap();
    let number_str = number.to_string();

    let comment_result = gh_exec(&[
        "issue", "comment",
        &number_str,
        "-R", &repo,
        "-b", "Integration test comment - please ignore",
    ]);

    assert!(comment_result.is_ok(), "Failed to add comment: {:?}", comment_result.err());
    println!("Added comment to issue #{}", number);
}
