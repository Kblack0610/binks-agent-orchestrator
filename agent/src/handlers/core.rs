//! Core command handlers - always available
//!
//! These handlers work without any feature flags.

use anyhow::Result;
use std::io::{self, BufRead, Write};

use super::CommandContext;
use crate::llm::{self, Llm, OllamaClient};

/// Handle the `chat` command - single message to LLM
pub async fn chat(ctx: &CommandContext, message: &str) -> Result<()> {
    let llm = ctx.llm();
    let response = llm.chat(message).await?;
    println!("{}", response);
    Ok(())
}

/// Handle the `models` command - list available models
pub async fn models(ctx: &CommandContext) -> Result<()> {
    let models = llm::list_models(&ctx.ollama_url).await?;
    println!("Available models:");
    for m in models {
        let current_marker = if m.name == ctx.model { " (current)" } else { "" };
        println!("  {}{}", m.name, current_marker);
    }
    Ok(())
}

/// Handle the `simple` command - interactive chat without tools
pub async fn simple(ctx: &CommandContext) -> Result<()> {
    let llm = ctx.llm();
    run_simple_loop(llm).await
}

/// Run the simple chat loop
async fn run_simple_loop(llm: OllamaClient) -> Result<()> {
    println!("Simple chat mode (no tools). Type 'quit' to exit.");
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
