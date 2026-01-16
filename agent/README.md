# Minimal Rust Agent

A bare-bones Rust agent with Ollama LLM and MCP (Model Context Protocol) support.

## Features

- **Ollama Integration**: Chat with local LLMs via Ollama
- **MCP Client**: Connect to MCP servers (kubernetes, ssh, github, etc.)
- **Single Binary**: Minimal dependencies, compiles to ~5MB stripped binary
- **Extensible**: Foundation for building a full agent system

## Quick Start

```bash
# Build
cargo build

# Chat with Ollama
cargo run -- --ollama-url http://localhost:11434 chat "Hello"

# Interactive chat session
cargo run -- interactive

# List available MCP tools
cargo run -- tools

# Call a specific tool
cargo run -- call <tool_name> --args '{"key": "value"}'
```

## Commands

| Command | Description |
|---------|-------------|
| `chat <message>` | Send a single message to the LLM |
| `interactive` | Start an interactive chat session |
| `agent [message]` | Run tool-using agent (LLM decides when to call tools) |
| `agent -s "prompt"` | Agent with custom system prompt |
| `tools` | List all available tools from MCP servers |
| `tools --server <name>` | List tools from a specific MCP server |
| `call <tool> [--args <json>]` | Call a tool directly |

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OLLAMA_URL` | `http://localhost:11434` | Ollama server URL |
| `OLLAMA_MODEL` | `llama3.1:8b` | Model to use |
| `RUST_LOG` | - | Log level (e.g., `info`, `debug`) |

### MCP Servers

The agent reads MCP server configuration from `.mcp.json` in the current directory or any parent directory:

```json
{
  "mcpServers": {
    "kubernetes": {
      "command": "npx",
      "args": ["-y", "kubernetes-mcp-server@latest"],
      "env": {
        "KUBECONFIG": "${HOME}/.kube/config"
      }
    },
    "ssh": {
      "command": "npx",
      "args": ["@aiondadotcom/mcp-ssh"]
    }
  }
}
```

## Architecture

```
agent/
├── Cargo.toml           # Dependencies (~13 crates)
├── src/
│   ├── main.rs          # CLI entry point (clap)
│   ├── lib.rs           # Module exports
│   ├── config.rs        # .mcp.json configuration loader
│   ├── agent/
│   │   └── mod.rs       # Tool-using agent loop
│   ├── llm/
│   │   ├── mod.rs       # Llm trait abstraction
│   │   └── ollama.rs    # Ollama implementation
│   └── mcp/
│       ├── mod.rs       # MCP module exports
│       └── client.rs    # MCP client pool
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ollama-rs` | Ollama API client |
| `rmcp` | MCP protocol (client + server) |
| `tokio` | Async runtime |
| `clap` | CLI argument parsing |
| `serde` / `serde_json` | Serialization |
| `anyhow` / `thiserror` | Error handling |
| `tracing` | Logging |

## Development

```bash
# Hot reload during development
cargo install cargo-watch
cargo watch -x 'run -- tools'

# Build release binary
cargo build --release

# Run with debug logging
RUST_LOG=debug cargo run -- tools
```

## Tool-Calling Models

For the `agent` command to work properly, you need an Ollama model that supports tool calling. Recommended models:

```bash
# Pull a model with good tool support
ollama pull qwen2.5:7b
ollama pull llama3.2:3b
ollama pull mistral:7b

# Use it
cargo run -- --model qwen2.5:7b agent "List my kubernetes namespaces"
```

## Roadmap

- [x] Phase 1: Ollama chat integration
- [x] Phase 2: MCP client (connect to servers, list tools)
- [x] Phase 3: Tool-using agent loop (LLM decides when to use tools)
- [ ] Phase 4: MCP server mode (expose agent as MCP server)

## License

MIT
