use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use agent::agent::Agent;
use agent::llm::{Llm, OllamaClient};
use agent::mcp::McpClientPool;
use agent::server::{self, ServerConfig};

#[derive(Parser)]
#[command(name = "agent")]
#[command(about = "Minimal Rust agent with Ollama and MCP support")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Ollama server URL
    #[arg(long, env = "OLLAMA_URL", default_value = "http://192.168.1.4:11434")]
    ollama_url: String,

    /// Model to use
    #[arg(long, env = "OLLAMA_MODEL", default_value = "llama3.1:8b")]
    model: String,
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Chat { message } => {
            let llm = OllamaClient::new(&cli.ollama_url, &cli.model);
            let response = llm.chat(&message).await?;
            println!("{}", response);
        }
        Commands::Interactive => {
            let llm = OllamaClient::new(&cli.ollama_url, &cli.model);
            run_interactive(llm).await?;
        }
        Commands::Agent { message, system } => {
            run_agent(&cli.ollama_url, &cli.model, message, system).await?;
        }
        Commands::Tools { server } => {
            run_tools(server).await?;
        }
        Commands::Call { tool, args } => {
            run_call_tool(&tool, args).await?;
        }
        Commands::Serve { system } => {
            let config = ServerConfig {
                ollama_url: cli.ollama_url,
                model: cli.model,
                system_prompt: system,
            };
            server::serve(config).await?;
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
) -> Result<()> {
    let pool = McpClientPool::load()?
        .ok_or_else(|| anyhow::anyhow!("No .mcp.json found - agent needs MCP tools"))?;

    let mut agent = Agent::new(ollama_url, model, pool);

    if let Some(sys) = system {
        agent = agent.with_system_prompt(&sys);
    }

    // Get available tools for display
    let tool_names = agent.tool_names().await?;
    println!("Agent mode with {} tools available", tool_names.len());
    println!("Model: {}", model);
    println!();

    if let Some(msg) = message {
        // Single message mode
        println!("> {}\n", msg);
        match agent.chat(&msg).await {
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

        println!("Interactive agent mode. Type 'quit' to exit, 'tools' to list tools.");
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

            if input == "clear" {
                agent.clear_history();
                println!("History cleared.\n");
                continue;
            }

            match agent.chat(input).await {
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
