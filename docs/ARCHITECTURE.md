# Binks Architecture

## Overview

Binks is a **Rust-based AI agent system** that uses the **Model Context Protocol (MCP)** for tool integration. The agent connects to Ollama for LLM capabilities and orchestrates tools exposed by MCP servers.

## Design Principles

1. **MCP-First** - All tools are exposed via MCP protocol for standardization
2. **Modular MCPs** - Each MCP server is independent, versioned, and composable
3. **Rust Core** - Performance and safety through Rust implementation
4. **Local LLM** - Ollama integration for privacy and control

## Architectural Anti-Patterns (DO NOT ADD)

The following features are **explicitly prohibited** from this codebase. They add hidden complexity, make debugging harder, and introduce subtle bugs:

### ❌ Retries / Exponential Backoff
- If something fails, it should fail immediately and clearly
- Retries hide transient issues and make timing bugs hard to reproduce
- Users should see failures and decide whether to retry

### ❌ Model Fallback Chains
- If the configured model isn't available, fail explicitly
- Silent fallback masks configuration issues
- Users should know which model is running

### ❌ Circuit Breakers
- Added complexity with minimal benefit for this use case
- Just let it fail and show the error

### ❌ Automatic Recovery Mechanisms
- Self-healing systems are hard to debug
- Prefer explicit failure over implicit recovery

### ❌ Rate Limiting with Backoff
- Let the underlying services (Ollama, MCP servers) handle this
- Don't add layers of abstraction

### ✅ DO Add These (Simple, Explicit)

- **Timeouts** - Simple, deterministic, prevents hangs
- **Configurable limits** - MAX_ITERATIONS, context size
- **Clear error messages** - Tell the user exactly what failed
- **Health checks** - Verify services are running before starting

### Design Philosophy

**Fail fast, fail loud.** This agent should be predictable and debuggable. When something breaks:
1. The error should be immediate
2. The error message should be clear
3. The user can decide what to do

Hidden recovery mechanisms make the system "feel" more stable but actually make it harder to diagnose real issues.

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         RUST AGENT                                   │
│                        (agent/src/)                                  │
│                                                                      │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │                    Agent Loop                                 │  │
│   │  1. Receive user message                                      │  │
│   │  2. Send to LLM with available tools                          │  │
│   │  3. If tool call → execute via MCP → feed result back         │  │
│   │  4. Repeat until LLM responds without tool calls              │  │
│   └──────────────────────────────────────────────────────────────┘  │
│                                                                      │
│   ┌─────────────────┐         ┌─────────────────────────────────┐   │
│   │   LLM Client    │         │        MCP Client Pool          │   │
│   │   (Ollama)      │         │  - Loads .mcp.json config       │   │
│   │                 │         │  - Spawns MCP server processes  │   │
│   │                 │         │  - Aggregates tools from all    │   │
│   └─────────────────┘         └─────────────────────────────────┘   │
│                                           │                          │
└───────────────────────────────────────────┼──────────────────────────┘
                                            │
                        MCP Protocol (stdio JSON-RPC)
                                            │
         ┌──────────────┬───────────────────┼───────────────┬──────────┐
         │              │                   │               │          │
         ▼              ▼                   ▼               ▼          ▼
    ┌─────────┐   ┌──────────┐       ┌──────────┐    ┌─────────┐  ┌────────┐
    │ sysinfo │   │ github-gh│       │kubernetes│    │   ssh   │  │  ...   │
    │   mcp   │   │   mcp    │       │   mcp    │    │   mcp   │  │        │
    └─────────┘   └──────────┘       └──────────┘    └─────────┘  └────────┘
      (Rust)        (Rust)             (Node)          (Node)
```

---

## Components

### 1. Rust Agent (`agent/`)

The core agent that orchestrates LLM interactions and tool usage.

**Key Modules:**
- `main.rs` - CLI entry point with subcommands
- `config.rs` - Loads `.mcp.json` configuration
- `agent/mod.rs` - Tool-using agent loop
- `llm/ollama.rs` - Ollama client implementation
- `mcp/client.rs` - MCP client pool for tool management

**Capabilities:**
- Interactive chat mode
- Tool-using agent mode (autonomous tool execution)
- MCP server mode (expose agent as an MCP server)

### 2. MCP Servers (`mcps/`)

Standalone servers that expose tools via the MCP protocol.

| Server | Language | Tools |
|--------|----------|-------|
| `sysinfo-mcp` | Rust | OS info, CPU, memory, disk, network, uptime |
| `github-gh` | Rust | Issues, PRs, workflows, repos via `gh` CLI |

Each MCP server:
- Runs as a subprocess
- Communicates via stdio (JSON-RPC)
- Exposes tools with JSON schemas
- Can be developed/versioned independently

### 3. Configuration (`.mcp.json`)

Defines available MCP servers:

```json
{
  "mcpServers": {
    "sysinfo": {
      "command": "./mcps/sysinfo-mcp/target/release/sysinfo-mcp"
    },
    "github-gh": {
      "command": "./mcps/github-gh/target/release/github-gh-mcp",
      "env": { "RUST_LOG": "info" }
    },
    "kubernetes": {
      "command": "npx",
      "args": ["-y", "kubernetes-mcp-server@latest"],
      "env": { "KUBECONFIG": "${HOME}/.kube/config" }
    }
  }
}
```

---

## Data Flow

### Agent Loop

```
User Message
     │
     ▼
┌─────────────────────────────────────┐
│  LLM (Ollama)                       │
│  - Receives message + tool schemas  │
│  - Decides: respond or use tools    │
└─────────────────┬───────────────────┘
                  │
        ┌─────────┴─────────┐
        │                   │
        ▼                   ▼
   [Tool Call]         [Response]
        │                   │
        ▼                   │
┌───────────────┐           │
│  MCP Server   │           │
│  Execute Tool │           │
└───────┬───────┘           │
        │                   │
        ▼                   │
   [Tool Result]            │
        │                   │
        └───────┬───────────┘
                │
                ▼
         Back to LLM
      (loop until done)
```

### Tool Discovery

1. Agent starts → reads `.mcp.json`
2. For each server: spawn process, initialize MCP connection
3. Call `tools/list` on each server → collect tool schemas
4. Convert to Ollama tool format
5. Tools available for LLM to call

---

## Directory Structure

```
binks-agent-orchestrator/
├── agent/                    # Rust agent
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # CLI entry
│       ├── config.rs         # .mcp.json loader
│       ├── agent/            # Agent loop
│       ├── llm/              # LLM clients
│       └── mcp/              # MCP client pool
│
├── mcps/                     # MCP servers
│   ├── sysinfo-mcp/          # System info (Rust)
│   └── github-gh/            # GitHub CLI (Rust)
│
├── test/                     # Integration tests
├── manifests/                # K8s deployment
├── scripts/                  # Utility scripts
├── model/                    # Ollama model scripts
├── docs/                     # Documentation
│
├── .mcp.json                 # MCP server configuration
└── README.md
```

---

## Adding New MCP Servers

### 1. Create the Server

```bash
mkdir mcps/my-new-mcp
cd mcps/my-new-mcp
cargo init
```

### 2. Implement Tools

Use `rmcp` crate with `#[tool_router]` macro:

```rust
#[tool_router]
impl MyMcpServer {
    #[tool(description = "Does something useful")]
    async fn my_tool(&self, params: MyParams) -> Result<CallToolResult, McpError> {
        // Implementation
    }
}
```

### 3. Register in `.mcp.json`

```json
{
  "mcpServers": {
    "my-new-mcp": {
      "command": "./mcps/my-new-mcp/target/release/my-new-mcp"
    }
  }
}
```

---

## Key Technologies

| Component | Technology |
|-----------|------------|
| Agent | Rust, tokio, rmcp |
| LLM | Ollama (local) |
| Protocol | MCP (Model Context Protocol) |
| MCPs | Rust (rmcp) or Node.js |
| Config | JSON (.mcp.json) |

---

## References

- [Model Context Protocol](https://modelcontextprotocol.io/)
- [rmcp Rust crate](https://crates.io/crates/rmcp)
- [Ollama](https://ollama.ai/)
- [Legacy Architecture](./LEGACY_ORCHESTRATION.md) - Historical Python-based design
