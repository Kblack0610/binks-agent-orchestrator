use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use agent::llm::{Llm, OllamaClient};

#[derive(Parser)]
#[command(name = "agent")]
#[command(about = "Minimal Rust agent with Ollama and MCP support")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Ollama server URL
    #[arg(long, env = "OLLAMA_URL", default_value = "http://localhost:11434")]
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
    /// List available tools (future: from MCP servers)
    Tools,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Create Ollama client
    let llm = OllamaClient::new(&cli.ollama_url, &cli.model);

    match cli.command {
        Commands::Chat { message } => {
            let response = llm.chat(&message).await?;
            println!("{}", response);
        }
        Commands::Interactive => {
            run_interactive(llm).await?;
        }
        Commands::Tools => {
            println!("Tool listing not yet implemented (Phase 2)");
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
