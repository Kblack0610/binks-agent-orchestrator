# Minimal Rust Agent - Implementation Plan

## Goal
Create a bare-bones `/agent` directory in Rust that:
- Connects to Ollama for LLM inference
- Acts as MCP client (connect to kubernetes, ssh, github MCPs)
- Acts as MCP server (expose agent to other tools)
- Single binary, minimal dependencies
- Foundation for growth

## Key Finding: Rust is Viable
- `ollama-rs` v0.3+ - production-ready Ollama client
- `rmcp` v0.13+ - official MCP SDK (already used in `mcps/github-gh/`)
- ~10 direct dependencies total
- Your existing `github-gh` MCP proves the pattern works

---

## Directory Structure

```
agent/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI
│   ├── lib.rs               # Library exports
│   ├── config.rs            # Load .mcp.json, env vars
│   ├── llm/
│   │   ├── mod.rs           # LLM trait
│   │   └── ollama.rs        # Ollama implementation
│   ├── mcp/
│   │   ├── mod.rs
│   │   ├── client.rs        # Connect to MCP servers
│   │   └── server.rs        # Expose agent as MCP
│   └── agent/
│       ├── mod.rs           # Core agent loop
│       ├── conversation.rs  # Message history
│       └── tools.rs         # Tool execution
└── README.md
```

---

## Phased Implementation

### Phase 1: Scaffold + Ollama Chat (2-4 hours)
**Goal**: CLI that chats with Ollama

1. `cargo new agent`
2. Add minimal deps to Cargo.toml
3. Implement basic chat loop with `ollama-rs`
4. Test: `./agent chat "Hello"`

```rust
// Minimal working example
let ollama = Ollama::default();
let response = ollama.send_chat_messages(...).await?;
```

### Phase 2: MCP Client (1-2 days)
**Goal**: Agent discovers and calls tools from .mcp.json servers

1. Parse `.mcp.json` config
2. Spawn MCP servers as child processes (pattern from github-gh)
3. List available tools from each server
4. Test: `./agent tools` lists kubernetes/ssh/github tools

### Phase 3: Tool-Using Agent Loop (2-3 days)
**Goal**: Agent autonomously decides when to use tools

1. Build system prompt with available tools
2. Implement think loop (chat → tool call → result → chat)
3. Test: `./agent task "list pods in production"`

### Phase 4: MCP Server Mode (1-2 days)
**Goal**: Expose agent as MCP server for Claude/other tools

1. Use `rmcp` server macros (same as github-gh)
2. Expose `agent/task` and `agent/chat` tools
3. Add to `.mcp.json`
4. Test: Claude CLI can call the agent

---

## Dependencies (Cargo.toml)

```toml
[package]
name = "agent"
version = "0.1.0"
edition = "2021"

[dependencies]
# LLM
ollama-rs = { version = "0.3", features = ["stream"] }

# MCP (client + server)
rmcp = { version = "0.13", features = ["server", "client", "macros", "transport-io"] }

# Async
tokio = { version = "1", features = ["full", "process"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1.2"

# Error handling
thiserror = "2"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
lto = true
strip = true
```

---

## Reference Files (Already in Repo)

| File | Use For |
|------|---------|
| `mcps/github-gh/Cargo.toml` | Dependency versions, build settings |
| `mcps/github-gh/src/server.rs` | MCP server pattern with rmcp macros |
| `mcps/github-gh/src/main.rs` | Entry point, tracing, stdio transport |
| `.mcp.json` | MCP server config format |

---

## Verification

After each phase:
1. **Phase 1**: `cargo run -- chat "What is 2+2?"` responds
2. **Phase 2**: `cargo run -- tools` lists tools from all MCP servers
3. **Phase 3**: `cargo run -- task "list k8s namespaces"` calls kubernetes MCP
4. **Phase 4**: Add to `.mcp.json`, verify Claude CLI sees agent tools

---

## Dev Workflow

```bash
# Hot reload during development
cargo install cargo-watch
cargo watch -x 'run -- chat "test"'

# With specific model
OLLAMA_MODEL=llama3.2 cargo run -- chat "Hello"
```

---

## Future: Extract to Own Repo

When ready:
1. Copy `/agent` to new repo
2. Update `.mcp.json` to point to new binary
3. Add CI/CD for releases
4. Architecture is self-contained, no orchestrator deps
