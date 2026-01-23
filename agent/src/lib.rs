//! Minimal Rust agent with Ollama and MCP support
//!
//! # Feature Flags
//!
//! - `mcp` - MCP tool support (agent tool-calling loop)
//! - `persistence` - SQLite conversation history
//! - `web` - Web UI server (requires persistence)
//! - `orchestrator` - Multi-agent workflows (requires mcp)
//! - `monitor` - Repository monitoring (requires mcp)
//!
//! # Build Profiles
//!
//! ```bash
//! # Minimal LLM-only chat
//! cargo build --no-default-features
//!
//! # Agent with MCP tools
//! cargo build --no-default-features --features mcp
//!
//! # Full featured (default)
//! cargo build
//! ```

// =============================================================================
// Core modules - always available
// =============================================================================
pub mod config;
pub mod context;
pub mod llm;
pub mod output;

// CLI module has mixed availability:
// - modes: always available
// - commands, repl: requires mcp feature
pub mod cli;

// =============================================================================
// Streaming module - requires "mcp" feature (uses AgentEvent)
// =============================================================================
#[cfg(feature = "mcp")]
pub mod streaming;

// =============================================================================
// MCP modules - requires "mcp" feature
// =============================================================================
#[cfg(feature = "mcp")]
pub mod agent;
#[cfg(feature = "mcp")]
pub mod mcp;
#[cfg(feature = "mcp")]
pub mod mcps;
#[cfg(feature = "mcp")]
pub mod server;

// =============================================================================
// Persistence module - requires "persistence" feature
// =============================================================================
#[cfg(feature = "persistence")]
pub mod db;

// =============================================================================
// Web module - requires "web" feature
// =============================================================================
#[cfg(feature = "web")]
pub mod web;

// =============================================================================
// Orchestrator module - requires "orchestrator" feature
// =============================================================================
#[cfg(feature = "orchestrator")]
pub mod orchestrator;

// =============================================================================
// Monitor module - requires "monitor" feature
// =============================================================================
#[cfg(feature = "monitor")]
pub mod monitor;
