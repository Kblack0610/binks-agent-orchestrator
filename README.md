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
./agent/target/release/agent tools

# Monitor repos (live view)
./agent/target/release/agent monitor --repos owner/repo --interval 60
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
| `github-gh` | GitHub CLI wrapper | 21 tools: issues, PRs, workflows, diffs, checks |
| `sysinfo-mcp` | Cross-platform system info | OS, CPU, memory, disk, network, uptime |
| `inbox-mcp` | Local file inbox | Write/read reports to ~/.notes/inbox |
| `notify-mcp` | Notifications | Slack, Discord webhooks |
| `kubernetes` | K8s management (external) | Pods, deployments, services |
| `ssh` | SSH operations (external) | Remote commands, file transfer |

See [mcps/overview.md](mcps/overview.md) for full tool list.

## Hardware

The Ollama LLM backend runs on a dedicated Mac Studio:

| Component | Specification |
|-----------|---------------|
| **Hostname** | Kenneths-Mac-Studio.local |
| **IP Address** | 192.168.1.4 |
| **OS** | macOS 15.6 (Build 24G84) |
| **CPU** | Apple M3 Ultra (32 cores) |
| **Memory** | 512 GB unified memory |
| **Storage** | 1.8 TB SSD (56% used) |
| **Ollama Port** | 11434 |

To connect to the Ollama host:
```bash
export OLLAMA_URL=http://192.168.1.4:11434
```

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

- [Agent CLI](agent/readme.md) - All agent commands and usage
- [MCP Servers](mcps/overview.md) - Available tools and how to add new servers
- [Monitoring](docs/monitoring.md) - Repository monitoring setup
- [Architecture](docs/ARCHITECTURE.md) - System design and components

## Adding New MCP Servers

1. Create a new Rust project in `mcps/`
2. Use `rmcp` crate with `#[tool_router]` macro
3. Add to `.mcp.json`

See [Architecture](docs/ARCHITECTURE.md#adding-new-mcp-servers) for details.

## License

MIT
