use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use agent::agent::Agent;
use agent::cli::{Repl, ReplConfig};
use agent::config::{AgentFileConfig, McpConfig};
use agent::context::EnvironmentContext;
use agent::llm::{Llm, OllamaClient};
use agent::mcp::McpClientPool;
use agent::mcps::{DaemonClient, McpDaemon, is_daemon_running, default_socket_path, default_pid_path, default_log_dir};
use agent::monitor::{self, MonitorConfig};
use agent::output::TerminalOutput;
use agent::server::{self, ServerConfig};
use agent::web::{self, WebConfig};

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
        /// Show timing information for each step
        #[arg(long, short)]
        verbose: bool,
    },
    /// List available tools from MCP servers
    Tools {
        /// Only list tools from a specific server
        #[arg(long)]
        server: Option<String>,
    },
    /// List available models from Ollama
    Models,
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
    /// MCP server management
    Mcps {
        #[command(subcommand)]
        command: McpsCommands,
    },
    /// Start the web interface server
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
}

#[derive(Subcommand)]
enum McpsCommands {
    /// Show MCP server status and tool cache
    Status {
        /// Show detailed tool list for each server
        #[arg(long, short)]
        verbose: bool,
    },
    /// Clear the tools cache and reconnect to servers
    Refresh,
    /// Start the MCP daemon (background supervisor for MCP servers)
    Start {
        /// Run as a background daemon process
        #[arg(long, short)]
        daemon: bool,
    },
    /// Stop the MCP daemon
    Stop,
    /// View daemon logs
    Logs {
        /// Number of lines to show (0 = all)
        #[arg(long, short, default_value = "50")]
        lines: usize,
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
        Commands::Agent { message, system, servers, verbose } => {
            let server_list = servers.map(|s| s.split(',').map(|s| s.trim().to_string()).collect());
            run_agent(&ollama_url, &model, message, system, server_list, verbose, &file_config).await?;
        }
        Commands::Tools { server } => {
            run_tools(server).await?;
        }
        Commands::Models => {
            let models = agent::llm::list_models(&ollama_url).await?;
            println!("Available models:");
            for m in models {
                let current_marker = if m.name == model { " (current)" } else { "" };
                println!("  {}{}", m.name, current_marker);
            }
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
        Commands::Mcps { command } => {
            run_mcps_command(command).await?;
        }
        Commands::Web { port, system, dev, open } => {
            let config = WebConfig {
                port,
                ollama_url: ollama_url.clone(),
                model: model.clone(),
                system_prompt: system.or_else(|| file_config.agent.system_prompt.clone()),
                dev_mode: dev,
            };

            if open {
                // Open browser after a short delay
                let url = format!("http://localhost:{}", port);
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    if let Err(e) = open::that(&url) {
                        tracing::warn!("Failed to open browser: {}", e);
                    }
                });
            }

            web::serve(config).await?;
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
    verbose: bool,
    file_config: &AgentFileConfig,
) -> Result<()> {
    let pool = McpClientPool::load()?
        .ok_or_else(|| anyhow::anyhow!("No .mcp.json found - agent needs MCP tools"))?;

    let mut agent = Agent::new(ollama_url, model, pool).with_verbose(verbose);

    // System prompt precedence: CLI > config file > auto-generated
    let system_prompt = system
        .or_else(|| file_config.agent.system_prompt.clone())
        .or_else(|| {
            let ctx = EnvironmentContext::gather();
            Some(ctx.to_system_prompt())
        });

    if let Some(sys) = system_prompt {
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
        // Interactive mode with enhanced REPL
        let output = TerminalOutput::auto();

        let mut repl_config = ReplConfig::default();
        if let Some(filter) = servers {
            repl_config.server_filter = Some(filter);
        }

        let mut repl = Repl::new(&mut agent, &output).with_config(repl_config);
        repl.run().await?;
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

async fn run_mcps_command(command: McpsCommands) -> Result<()> {
    match command {
        McpsCommands::Status { verbose } => {
            run_mcps_status(verbose).await
        }
        McpsCommands::Refresh => {
            run_mcps_refresh().await
        }
        McpsCommands::Start { daemon } => {
            run_mcps_start(daemon).await
        }
        McpsCommands::Stop => {
            run_mcps_stop().await
        }
        McpsCommands::Logs { lines } => {
            run_mcps_logs(lines).await
        }
    }
}

async fn run_mcps_status(verbose: bool) -> Result<()> {
    println!("=== MCP Server Status ===\n");

    let pool = McpClientPool::load()?;

    match pool {
        Some(mut pool) => {
            let servers = pool.server_names();

            if servers.is_empty() {
                println!("No MCP servers configured.");
                return Ok(());
            }

            println!("Configured servers: {}\n", servers.len());

            for server in &servers {
                print!("  {} ", server);

                // Check if tools are cached
                let cached = pool.has_cached_tools(server);

                // Try to get tools (this will cache them if not already cached)
                match pool.list_tools_from(server).await {
                    Ok(tools) => {
                        let cache_status = if cached { "(cached)" } else { "(fresh)" };
                        println!("✓ {} tools {}", tools.len(), cache_status);

                        if verbose {
                            for tool in &tools {
                                let desc = tool.description
                                    .as_deref()
                                    .unwrap_or("No description")
                                    .lines()
                                    .next()
                                    .unwrap_or("");
                                // Truncate long descriptions
                                let desc = if desc.len() > 60 {
                                    format!("{}...", &desc[..60])
                                } else {
                                    desc.to_string()
                                };
                                println!("      - {} : {}", tool.name, desc);
                            }
                        }
                    }
                    Err(e) => {
                        println!("✗ Failed: {}", e);
                    }
                }
            }

            // Summary
            let all_tools = pool.list_all_tools().await?;
            println!("\nTotal: {} tools across {} servers", all_tools.len(), servers.len());
        }
        None => {
            println!("No .mcp.json found in current directory.");
            println!("Create one to configure MCP servers.");
        }
    }

    Ok(())
}

async fn run_mcps_refresh() -> Result<()> {
    println!("Refreshing MCP connections...\n");

    let pool = McpClientPool::load()?;

    match pool {
        Some(mut pool) => {
            // Clear cache
            pool.clear_cache();
            println!("Cache cleared.");

            // Reconnect to all servers
            let servers = pool.server_names();
            let mut success = 0;
            let mut failed = 0;

            for server in &servers {
                print!("  {} ", server);
                match pool.list_tools_from(server).await {
                    Ok(tools) => {
                        println!("✓ {} tools", tools.len());
                        success += 1;
                    }
                    Err(e) => {
                        println!("✗ {}", e);
                        failed += 1;
                    }
                }
            }

            println!("\nRefresh complete: {} succeeded, {} failed", success, failed);
        }
        None => {
            println!("No .mcp.json found.");
        }
    }

    Ok(())
}

async fn run_mcps_start(daemon: bool) -> Result<()> {
    // Check if daemon is already running
    if is_daemon_running().await {
        println!("MCP daemon is already running.");
        println!("Socket: {:?}", default_socket_path());
        return Ok(());
    }

    // Load MCP config
    let config = McpConfig::load()?
        .ok_or_else(|| anyhow::anyhow!("No .mcp.json found"))?;

    let socket_path = default_socket_path();
    let pid_path = default_pid_path();
    let log_dir = default_log_dir();

    // Ensure directories exist
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::create_dir_all(&log_dir).await?;

    if daemon {
        // Fork to background using std::process::Command
        println!("Starting MCP daemon in background...");

        let log_file = log_dir.join("daemon.log");
        let current_exe = std::env::current_exe()?;
        let current_dir = std::env::current_dir()?;

        // Re-execute ourselves with mcps start (without --daemon)
        let child = std::process::Command::new(&current_exe)
            .arg("mcps")
            .arg("start")
            .current_dir(&current_dir)
            .stdout(std::fs::File::create(&log_file)?)
            .stderr(std::fs::File::create(log_dir.join("daemon.err"))?)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn daemon: {}", e))?;

        // Write PID file
        let pid = child.id();
        tokio::fs::write(&pid_path, pid.to_string()).await?;

        // Wait briefly and check if daemon started
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        if is_daemon_running().await {
            println!("MCP daemon started successfully.");
            println!("  PID: {}", pid);
            println!("  Socket: {:?}", socket_path);
            println!("  Log: {:?}", log_file);
        } else {
            println!("Warning: Daemon may not have started. Check logs:");
            println!("  {:?}", log_file);
        }
    } else {
        // Run in foreground
        println!("Starting MCP daemon (foreground)...");
        println!("  Socket: {:?}", socket_path);
        println!("  Press Ctrl+C to stop.\n");

        // Write PID file for foreground mode too
        let pid = std::process::id();
        tokio::fs::write(&pid_path, pid.to_string()).await?;

        let daemon = McpDaemon::new(config, socket_path);
        daemon.run().await?;

        // Cleanup PID file
        let _ = tokio::fs::remove_file(&pid_path).await;
    }

    Ok(())
}

async fn run_mcps_stop() -> Result<()> {
    let socket_path = default_socket_path();
    let pid_path = default_pid_path();

    // First try to send shutdown command via socket
    if is_daemon_running().await {
        println!("Sending shutdown command to daemon...");
        let client = DaemonClient::with_socket_path(socket_path.clone());
        match client.shutdown().await {
            Ok(_) => {
                println!("Daemon shutdown initiated.");
            }
            Err(e) => {
                println!("Warning: shutdown command failed: {}", e);
            }
        }

        // Wait a moment for clean shutdown
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Check if PID file exists and kill process if needed
    if pid_path.exists() {
        let pid_str = tokio::fs::read_to_string(&pid_path).await?;
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            // Check if process is still running
            #[cfg(unix)]
            {
                // Try to send SIGTERM
                let _ = unsafe { libc::kill(pid, libc::SIGTERM) };
            }
        }

        // Remove PID file
        let _ = tokio::fs::remove_file(&pid_path).await;
    }

    // Remove socket file if it exists
    let _ = tokio::fs::remove_file(&socket_path).await;

    println!("MCP daemon stopped.");
    Ok(())
}

async fn run_mcps_logs(lines: usize) -> Result<()> {
    let log_dir = default_log_dir();
    let log_file = log_dir.join("daemon.log");
    let err_file = log_dir.join("daemon.err");

    if !log_file.exists() && !err_file.exists() {
        println!("No daemon logs found.");
        println!("Expected location: {:?}", log_dir);
        return Ok(());
    }

    // Read stdout log
    if log_file.exists() {
        println!("=== Daemon stdout ({:?}) ===\n", log_file);
        let content = tokio::fs::read_to_string(&log_file).await?;
        let log_lines: Vec<&str> = content.lines().collect();

        let display_lines = if lines == 0 {
            &log_lines[..]
        } else {
            let start = log_lines.len().saturating_sub(lines);
            &log_lines[start..]
        };

        for line in display_lines {
            println!("{}", line);
        }
    }

    // Read stderr log
    if err_file.exists() {
        let err_content = tokio::fs::read_to_string(&err_file).await?;
        if !err_content.trim().is_empty() {
            println!("\n=== Daemon stderr ({:?}) ===\n", err_file);
            let err_lines: Vec<&str> = err_content.lines().collect();

            let display_lines = if lines == 0 {
                &err_lines[..]
            } else {
                let start = err_lines.len().saturating_sub(lines);
                &err_lines[start..]
            };

            for line in display_lines {
                println!("{}", line);
            }
        }
    }

    Ok(())
}
