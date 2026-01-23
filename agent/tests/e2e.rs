//! E2E Tests for the Binks Agent
//!
//! These tests require:
//! - Ollama running locally (default: http://localhost:11434)
//! - A model pulled (default: llama3.1:8b)
//! - Workspace built (cargo build --workspace)
//!
//! Run with: cargo test --test e2e -- --include-ignored
//!
//! Test structure:
//! - prerequisites: Verify Ollama and binaries available
//! - health_check: Test the `agent health` command
//! - tool_execution: Test direct MCP tool calls

#[path = "e2e/prerequisites.rs"]
mod prerequisites;

#[path = "e2e/health_check.rs"]
mod health_check;

#[path = "e2e/tool_execution.rs"]
mod tool_execution;

#[path = "e2e/directory_independence.rs"]
mod directory_independence;
