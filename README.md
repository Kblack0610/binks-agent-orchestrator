# Binks Agent Orchestrator

A **Rust-based AI agent system** using the **Model Context Protocol (MCP)** for modular tool integration, powered by local LLMs via Ollama.

## Overview

Binks is an orchestration platform that connects an AI agent to various tools through MCP servers. The agent can autonomously use tools to accomplish tasks, from querying system information to managing GitHub repositories.

## Architecture

```
┌──────────────────┐     ┌──────────────────┐
│   Rust Agent     │────▶│   Ollama LLM     │
│  (tool-using)    │     │                  │
└────────┬─────────┘     └──────────────────┘
         │
         │ MCP Protocol
         │
    ┌────┴────┬─────────┬─────────┐
    ▼         ▼         ▼         ▼
┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
│sysinfo│ │github │ │  k8s  │ │  ssh  │
│  mcp  │ │-gh mcp│ │  mcp  │ │  mcp  │
└───────┘ └───────┘ └───────┘ └───────┘
```

## Quick Start

### Prerequisites

- Rust (latest stable)
- Ollama with a model installed (e.g., `ollama pull llama3.1:8b`)
- `gh` CLI (for GitHub MCP)

### Build

```bash
# Build the agent
cd agent && cargo build --release

# Build MCP servers
cd mcps/sysinfo-mcp && cargo build --release
cd mcps/github-gh && cargo build --release
```

### Run

```bash
# Interactive agent mode
./agent/target/release/agent agent "What is my system info?"

# Chat mode (no tools)
./agent/target/release/agent chat

# List available tools
./agent/target/release/agent list-tools
```

## Project Structure

```
binks-agent-orchestrator/
├── agent/              # Rust agent (LLM + MCP client)
├── mcps/               # MCP servers
│   ├── sysinfo-mcp/    # System information tools
│   └── github-gh/      # GitHub CLI tools
├── test/               # Integration tests
├── manifests/          # K8s deployment
├── scripts/            # Utility scripts
├── model/              # Ollama model scripts
├── docs/               # Documentation
├── .mcp.json           # MCP server configuration
└── README.md
```

## MCP Servers

| Server | Description | Tools |
|--------|-------------|-------|
| `sysinfo-mcp` | Cross-platform system info | OS, CPU, memory, disk, network, uptime |
| `github-gh` | GitHub CLI wrapper | Issues, PRs, workflows, repos |
| `kubernetes` | K8s management (external) | Pods, deployments, services |
| `ssh` | SSH operations (external) | Remote commands, file transfer |

## Configuration

MCP servers are configured in `.mcp.json`:

```json
{
  "mcpServers": {
    "sysinfo": {
      "command": "./mcps/sysinfo-mcp/target/release/sysinfo-mcp"
    },
    "github-gh": {
      "command": "./mcps/github-gh/target/release/github-gh-mcp"
    }
  }
}
```

Environment variables for the agent:
```bash
export OLLAMA_URL=http://localhost:11434
export OLLAMA_MODEL=llama3.1:8b
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md) - System design and components
- [Legacy Orchestration](docs/LEGACY_ORCHESTRATION.md) - Historical Python-based design

## Adding New MCP Servers

1. Create a new Rust project in `mcps/`
2. Use `rmcp` crate with `#[tool_router]` macro
3. Add to `.mcp.json`

See [Architecture](docs/ARCHITECTURE.md#adding-new-mcp-servers) for details.

## License

MIT
