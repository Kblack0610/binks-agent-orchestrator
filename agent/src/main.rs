use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use agent::agent::Agent;
use agent::config::{AgentFileConfig, McpConfig};
use agent::llm::{Llm, OllamaClient};
use agent::mcp::McpClientPool;
use agent::monitor::{self, MonitorConfig};
use agent::server::{self, ServerConfig};

#[derive(Parser)]
#[command(name = "agent")]
#[command(about = "Minimal Rust agent with Ollama and MCP support")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Ollama server URL (default: from .agent.toml or http://localhost:11434)
    #[arg(long, env = "OLLAMA_URL")]
    ollama_url: Option<String>,

    /// Model to use (default: from .agent.toml or qwen2.5-coder:32b)
    #[arg(long, env = "OLLAMA_MODEL")]
    model: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Chat with the LLM
    Chat {
        /// Message to send
        message: String,
    },
    /// Interactive chat session
    Interactive,
    /// Run the tool-using agent (LLM decides when to call tools)
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
    Tools {
        /// Only list tools from a specific server
        #[arg(long)]
        server: Option<String>,
    },
    /// Call a tool directly
    Call {
        /// Tool name
        tool: String,
        /// Arguments as JSON
        #[arg(long, short)]
        args: Option<String>,
    },
    /// Run as an MCP server (expose agent as MCP tools)
    Serve {
        /// System prompt for the agent
        #[arg(long, short)]
        system: Option<String>,
    },
    /// Run the monitoring agent (poll repos, write to inbox, send notifications)
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
    /// Run health checks on agent components
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load config file (.agent.toml) - returns defaults if not found
    let file_config = AgentFileConfig::load()?;

    let cli = Cli::parse();

    // Resolve final values with priority: CLI/env > config file > hardcoded defaults
    let ollama_url = cli.ollama_url.unwrap_or_else(|| file_config.llm.url.clone());
    let model = cli.model.unwrap_or_else(|| file_config.llm.model.clone());

    match cli.command {
        Commands::Chat { message } => {
            let llm = OllamaClient::new(&ollama_url, &model);
            let response = llm.chat(&message).await?;
            println!("{}", response);
        }
        Commands::Interactive => {
            let llm = OllamaClient::new(&ollama_url, &model);
            run_interactive(llm).await?;
        }
        Commands::Agent { message, system, servers } => {
            let server_list = servers.map(|s| s.split(',').map(|s| s.trim().to_string()).collect());
            run_agent(&ollama_url, &model, message, system, server_list).await?;
        }
        Commands::Tools { server } => {
            run_tools(server).await?;
        }
        Commands::Call { tool, args } => {
            run_call_tool(&tool, args).await?;
        }
        Commands::Serve { system } => {
            let config = ServerConfig {
                ollama_url: ollama_url,
                model: model,
                system_prompt: system,
            };
            server::serve(config).await?;
        }
        Commands::Monitor { once, interval, repos, system } => {
            // Use repos from CLI, or fall back to config file
            let repos = repos
                .map(|r| r.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|| file_config.monitor.repos.clone());

            if repos.is_empty() {
                eprintln!("Error: No repositories specified. Use --repos to specify repos to monitor,");
                eprintln!("or set repos in .agent.toml under [monitor].");
                eprintln!("Example: agent monitor --once --repos owner/repo1,owner/repo2");
                std::process::exit(1);
            }

            // Use interval from CLI (300 default), but could also check config
            let config = MonitorConfig {
                ollama_url: ollama_url,
                model: model,
                repos,
                once,
                interval,
                system_prompt: system,
            };
            monitor::run_monitor(config).await?;
        }
        Commands::Health { test_llm, test_tools, all } => {
            run_health(&ollama_url, &model, test_llm || all, test_tools || all).await?;
        }
    }

    Ok(())
}

async fn run_interactive(llm: OllamaClient) -> Result<()> {
    use std::io::{self, BufRead, Write};

    println!("Interactive mode. Type 'quit' to exit.");
    println!("Model: {}", llm.model());
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut history = Vec::new();

    loop {
        print!("> ");
        stdout.flush()?;

        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "quit" || input == "exit" {
            break;
        }

        match llm.chat_with_history(&mut history, input).await {
            Ok(response) => {
                println!("\n{}\n", response);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}

async fn run_tools(server_filter: Option<String>) -> Result<()> {
    let pool = McpClientPool::load()?;

    match pool {
        Some(mut pool) => {
            println!("Loading MCP servers from .mcp.json...\n");

            let servers = pool.server_names();
            println!("Configured servers: {}\n", servers.join(", "));

            let tools = pool.list_all_tools().await?;

            if tools.is_empty() {
                println!("No tools found.");
                return Ok(());
            }

            // Filter by server if specified
            let tools: Vec<_> = match &server_filter {
                Some(s) => tools.into_iter().filter(|t| &t.server == s).collect(),
                None => tools,
            };

            // Group by server
            let mut by_server: std::collections::HashMap<String, Vec<_>> =
                std::collections::HashMap::new();
            for tool in tools {
                by_server
                    .entry(tool.server.clone())
                    .or_default()
                    .push(tool);
            }

            for (server, tools) in by_server {
                println!("=== {} ({} tools) ===", server, tools.len());
                for tool in tools {
                    let desc = tool
                        .description
                        .as_deref()
                        .unwrap_or("No description")
                        .lines()
                        .next()
                        .unwrap_or("");
                    println!("  {} - {}", tool.name, desc);
                }
                println!();
            }
        }
        None => {
            println!("No .mcp.json found in current directory or parents.");
            println!("Create one to configure MCP servers.");
        }
    }

    Ok(())
}

async fn run_call_tool(tool_name: &str, args: Option<String>) -> Result<()> {
    let mut pool = McpClientPool::load()?
        .ok_or_else(|| anyhow::anyhow!("No .mcp.json found"))?;

    let arguments = match args {
        Some(json) => Some(serde_json::from_str(&json)?),
        None => None,
    };

    println!("Calling tool: {}", tool_name);
    if let Some(ref a) = arguments {
        println!("Arguments: {}", serde_json::to_string_pretty(a)?);
    }
    println!();

    let result = pool.call_tool(tool_name, arguments).await?;

    println!("Result:");
    for content in &result.content {
        match &content.raw {
            rmcp::model::RawContent::Text(text) => {
                println!("{}", text.text);
            }
            _ => {
                println!("{:?}", content);
            }
        }
    }

    Ok(())
}

async fn run_agent(
    ollama_url: &str,
    model: &str,
    message: Option<String>,
    system: Option<String>,
    servers: Option<Vec<String>>,
) -> Result<()> {
    let pool = McpClientPool::load()?
        .ok_or_else(|| anyhow::anyhow!("No .mcp.json found - agent needs MCP tools"))?;

    let mut agent = Agent::new(ollama_url, model, pool);

    if let Some(sys) = system {
        agent = agent.with_system_prompt(&sys);
    }

    // Get available tools for display
    let tool_names = agent.tool_names().await?;
    let server_names = agent.server_names().await?;

    println!("Agent mode with {} tools from {} servers", tool_names.len(), server_names.len());
    println!("Servers: {}", server_names.join(", "));
    if let Some(ref filter) = servers {
        println!("Filtered to: {}", filter.join(", "));
    }
    println!("Model: {}", model);
    println!();

    // Create server filter as string slices
    let server_refs: Option<Vec<&str>> = servers.as_ref().map(|v| v.iter().map(|s| s.as_str()).collect());

    if let Some(msg) = message {
        // Single message mode
        println!("> {}\n", msg);
        let result = if let Some(ref srvs) = server_refs {
            agent.chat_with_servers(&msg, srvs).await
        } else {
            agent.chat(&msg).await
        };

        match result {
            Ok(response) => {
                println!("{}", response);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    } else {
        // Interactive mode
        use std::io::{self, BufRead, Write};

        println!("Interactive agent mode. Type 'quit' to exit, 'tools' to list tools, 'servers' to list servers.");
        println!();

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            print!("agent> ");
            stdout.flush()?;

            let mut input = String::new();
            stdin.lock().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            if input == "quit" || input == "exit" {
                break;
            }

            if input == "tools" {
                match agent.tool_names().await {
                    Ok(names) => {
                        println!("\nAvailable tools ({}):", names.len());
                        for name in names {
                            println!("  - {}", name);
                        }
                        println!();
                    }
                    Err(e) => {
                        eprintln!("Error listing tools: {}", e);
                    }
                }
                continue;
            }

            if input == "servers" {
                match agent.server_names().await {
                    Ok(names) => {
                        println!("\nAvailable servers ({}):", names.len());
                        for name in names {
                            println!("  - {}", name);
                        }
                        println!();
                    }
                    Err(e) => {
                        eprintln!("Error listing servers: {}", e);
                    }
                }
                continue;
            }

            if input == "clear" {
                agent.clear_history();
                println!("History cleared.\n");
                continue;
            }

            let result = if let Some(ref srvs) = server_refs {
                agent.chat_with_servers(input, srvs).await
            } else {
                agent.chat(input).await
            };

            match result {
                Ok(response) => {
                    println!("\n{}\n", response);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }
    }

    Ok(())
}

async fn run_health(ollama_url: &str, model: &str, test_llm: bool, test_tools: bool) -> Result<()> {
    println!("=== Agent Health Check ===\n");

    let mut all_passed = true;
    let mut checks_run = 0;
    let mut checks_passed = 0;

    // Helper to print status
    fn status(passed: bool) -> &'static str {
        if passed { "✓" } else { "✗" }
    }

    // 1. Check .agent.toml config
    checks_run += 1;
    print!("Config (.agent.toml): ");
    let config_path = std::env::current_dir()?.join(".agent.toml");
    if config_path.exists() {
        match AgentFileConfig::load() {
            Ok(config) => {
                println!("{} Found", status(true));
                println!("  - LLM URL: {}", config.llm.url);
                println!("  - Model: {}", config.llm.model);
                checks_passed += 1;
            }
            Err(e) => {
                println!("{} Parse error: {}", status(false), e);
                all_passed = false;
            }
        }
    } else {
        println!("{} Not found (using defaults)", status(true));
        println!("  - LLM URL: {}", ollama_url);
        println!("  - Model: {}", model);
        checks_passed += 1;
    }

    // 2. Check .mcp.json config
    checks_run += 1;
    print!("Config (.mcp.json): ");
    let mcp_path = std::env::current_dir()?.join(".mcp.json");
    if mcp_path.exists() {
        match McpConfig::load() {
            Ok(Some(config)) => {
                let server_count = config.mcp_servers.len();
                println!("{} Found ({} servers)", status(true), server_count);
                for name in config.mcp_servers.keys() {
                    println!("  - {}", name);
                }
                checks_passed += 1;
            }
            Ok(None) => {
                println!("{} Not found", status(false));
                all_passed = false;
            }
            Err(e) => {
                println!("{} Parse error: {}", status(false), e);
                all_passed = false;
            }
        }
    } else {
        println!("{} Not found", status(false));
        all_passed = false;
    }

    // 3. Check MCP server connections and tools
    checks_run += 1;
    print!("MCP Connections: ");
    match McpClientPool::load() {
        Ok(Some(mut pool)) => {
            match pool.list_all_tools().await {
                Ok(tools) => {
                    // Group by server
                    let mut by_server: std::collections::HashMap<String, usize> =
                        std::collections::HashMap::new();
                    for tool in &tools {
                        *by_server.entry(tool.server.clone()).or_default() += 1;
                    }

                    let connected_servers = by_server.len();
                    let total_tools = tools.len();
                    println!("{} {} servers, {} tools", status(true), connected_servers, total_tools);

                    for (server, count) in by_server.iter() {
                        println!("  - {}: {} tools", server, count);
                    }
                    checks_passed += 1;
                }
                Err(e) => {
                    println!("{} Tool discovery failed: {}", status(false), e);
                    all_passed = false;
                }
            }
        }
        Ok(None) => {
            println!("{} No .mcp.json found", status(false));
            all_passed = false;
        }
        Err(e) => {
            println!("{} Failed to load: {}", status(false), e);
            all_passed = false;
        }
    }

    // 4. Optional: Test LLM connectivity
    if test_llm {
        checks_run += 1;
        print!("LLM Connectivity: ");
        let llm = OllamaClient::new(ollama_url, model);
        match llm.chat("Say 'OK' and nothing else.").await {
            Ok(response) => {
                let trimmed = response.trim();
                let short_response = if trimmed.len() > 50 {
                    format!("{}...", &trimmed[..50])
                } else {
                    trimmed.to_string()
                };
                println!("{} Response: \"{}\"", status(true), short_response);
                checks_passed += 1;
            }
            Err(e) => {
                println!("{} Failed: {}", status(false), e);
                all_passed = false;
            }
        }
    }

    // 5. Optional: Test tool execution
    if test_tools {
        checks_run += 1;
        print!("Tool Execution: ");
        match McpClientPool::load() {
            Ok(Some(mut pool)) => {
                // Try to call a simple sysinfo tool (without the mcp__ prefix)
                match pool.call_tool("get_uptime", None).await {
                    Ok(result) => {
                        // Extract the text from result
                        let text = result.content.iter()
                            .filter_map(|c| match &c.raw {
                                rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
                                _ => None,
                            })
                            .next()
                            .unwrap_or("(no text)");

                        // Truncate if needed
                        let short_text = if text.len() > 60 {
                            format!("{}...", &text[..60])
                        } else {
                            text.to_string()
                        };
                        println!("{} get_uptime returned: {}", status(true), short_text);
                        checks_passed += 1;
                    }
                    Err(e) => {
                        println!("{} Failed: {}", status(false), e);
                        all_passed = false;
                    }
                }
            }
            Ok(None) => {
                println!("{} No MCP pool available", status(false));
                all_passed = false;
            }
            Err(e) => {
                println!("{} Pool load failed: {}", status(false), e);
                all_passed = false;
            }
        }
    }

    // Summary
    println!("\n=== Summary ===");
    println!("Checks: {}/{} passed", checks_passed, checks_run);

    if all_passed {
        println!("\nAll health checks passed!");
        Ok(())
    } else {
        println!("\nSome health checks failed.");
        std::process::exit(1);
    }
}
