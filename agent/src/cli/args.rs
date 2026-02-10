//! CLI argument definitions
//!
//! Contains the main CLI struct and Commands enum for clap parsing.

use clap::{ArgAction, Parser, Subcommand};

#[cfg(feature = "mcp")]
use super::mcps_args::McpsCommands;
#[cfg(feature = "orchestrator")]
use super::runs_args::RunsCommands;
#[cfg(feature = "orchestrator")]
use super::selfheal_args::SelfHealCommands;
#[cfg(feature = "orchestrator")]
use super::workflow_args::WorkflowCommands;

#[derive(Parser)]
#[command(name = "agent")]
#[command(about = "Minimal Rust agent with Ollama and MCP support")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Ollama server URL (default: from .agent.toml or http://localhost:11434)
    #[arg(long, env = "OLLAMA_URL", global = true)]
    pub ollama_url: Option<String>,

    /// Model to use (required: must be specified in .agent.toml or via -m flag)
    #[arg(short = 'm', long, env = "OLLAMA_MODEL", global = true)]
    pub model: Option<String>,

    /// Increase verbosity (-v info, -vv debug, -vvv trace). Default is warn.
    #[arg(short, long, action = ArgAction::Count, global = true)]
    pub verbose: u8,
}

#[derive(Subcommand)]
pub enum Commands {
    // =========================================================================
    // Core commands - always available
    // =========================================================================
    /// Chat with the LLM (single message)
    Chat {
        /// Message to send
        message: String,
    },
    /// Simple chat session (no tools, just LLM conversation)
    Simple,
    /// List available models from Ollama
    Models,

    // =========================================================================
    // MCP commands - requires "mcp" feature
    // =========================================================================
    /// Run the tool-using agent (LLM decides when to call tools)
    #[cfg(feature = "mcp")]
    Agent {
        /// Initial message (optional, starts interactive if not provided)
        message: Option<String>,
        /// System prompt for the agent
        #[arg(long, short)]
        system: Option<String>,
        /// Filter tools to specific MCP servers (comma-separated, e.g., "sysinfo,kubernetes")
        /// Recommended for smaller models that struggle with many tools
        #[arg(long)]
        servers: Option<String>,
    },
    /// List available tools from MCP servers
    #[cfg(feature = "mcp")]
    Tools {
        /// Only list tools from a specific server
        #[arg(long)]
        server: Option<String>,
    },
    /// Call a tool directly
    #[cfg(feature = "mcp")]
    Call {
        /// Tool name
        tool: String,
        /// Arguments as JSON
        #[arg(long, short)]
        args: Option<String>,
    },
    /// Run as an MCP server (expose agent as MCP tools)
    #[cfg(feature = "mcp")]
    Serve {
        /// System prompt for the agent
        #[arg(long, short)]
        system: Option<String>,
    },
    /// Run health checks on agent components
    #[cfg(feature = "mcp")]
    Health {
        /// Also test LLM connectivity with a simple query
        #[arg(long)]
        test_llm: bool,
        /// Also test tool execution with a simple call
        #[arg(long)]
        test_tools: bool,
        /// Run all tests (equivalent to --test-llm --test-tools)
        #[arg(long, short)]
        all: bool,
    },
    /// MCP server management
    #[cfg(feature = "mcp")]
    Mcps {
        #[command(subcommand)]
        command: McpsCommands,
    },

    // =========================================================================
    // Monitor commands - requires "monitor" feature
    // =========================================================================
    /// Run the monitoring agent (poll repos, write to inbox, send notifications)
    #[cfg(feature = "monitor")]
    Monitor {
        /// Run once instead of continuously
        #[arg(long)]
        once: bool,
        /// Polling interval in seconds (for continuous mode)
        #[arg(long, default_value = "300")]
        interval: u64,
        /// Repositories to monitor (comma-separated, e.g., "owner/repo1,owner/repo2")
        #[arg(long)]
        repos: Option<String>,
        /// System prompt for the agent
        #[arg(long, short)]
        system: Option<String>,
    },

    // =========================================================================
    // Web commands - requires "web" feature
    // =========================================================================
    /// Start the web interface server
    #[cfg(feature = "web")]
    Web {
        /// Port to listen on
        #[arg(short, long, default_value = "3001")]
        port: u16,
        /// System prompt for the agent
        #[arg(long, short)]
        system: Option<String>,
        /// Run in development mode (no embedded frontend)
        #[arg(long)]
        dev: bool,
        /// Open browser after starting
        #[arg(long)]
        open: bool,
    },

    // =========================================================================
    // Orchestrator commands - requires "orchestrator" feature
    // =========================================================================
    /// Run multi-agent workflows
    #[cfg(feature = "orchestrator")]
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommands,
    },
    /// View, analyze, and manage workflow runs
    #[cfg(feature = "orchestrator")]
    Runs {
        #[command(subcommand)]
        command: RunsCommands,
    },
    /// Analyze failures and apply automated improvements
    #[cfg(feature = "orchestrator")]
    SelfHeal {
        #[command(subcommand)]
        command: Option<SelfHealCommands>,
    },
}
