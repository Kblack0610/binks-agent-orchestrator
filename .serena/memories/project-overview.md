# Binks Agent Orchestrator - Project Overview

## What is this?
Rust-based AI agent orchestration system using Model Context Protocol (MCP) for modular tool integration. The agent autonomously uses tools to accomplish tasks via local LLM inference.

## Tech Stack
- **Language:** Rust 2021, tokio async runtime
- **LLM:** Ollama (local inference at 192.168.1.4:11434)
- **Model:** qwen2.5-coder:32b (primary), llama3.1:8b (fallback)
- **Protocol:** MCP - stdio JSON-RPC
- **Tool SDK:** rmcp 0.13
- **CLI:** clap 4.0

## MCP Servers (Tools)
| Server | Language | Purpose |
|--------|----------|---------|
| github-gh | Rust | GitHub CLI - issues, PRs, workflows |
| sysinfo | Rust | System info - CPU, memory, disk, network |
| inbox | Rust | Local file-based inbox (~/.notes/inbox/) |
| notify | Rust | Slack/Discord notifications |
| kubernetes | Node | K8s management |
| ssh | Node | SSH remote commands |

## CLI Commands
```bash
agent chat "message"      # Simple LLM chat (no tools)
agent agent "task"        # Tool-using autonomous agent
agent tools               # List available tools
agent serve               # Expose agent as MCP server
agent monitor             # Repository monitoring
```

## Configuration Files
- `.agent.toml` - Agent settings (LLM URL, model, repos)
- `.mcp.json` - MCP server definitions

## Key Directories
- `agent/` - Core Rust agent
- `mcps/` - MCP server implementations
- `docs/` - Architecture documentation

## Architecture Pattern
MCP-first design: All tools standardized via MCP protocol. Agent loop: User message → LLM decides tool calls → Execute tools → Feed results back → Repeat until done.
