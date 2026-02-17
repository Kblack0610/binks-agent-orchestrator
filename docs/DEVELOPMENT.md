# Development Guide

## Architecture Overview

| Component | Location | Port | Purpose |
|-----------|----------|------|---------|
| binks-agent-orchestrator | `~/dev/home/binks-agent-orchestrator` | 7317 | Rust backend (API + WebSocket) |
| binks-chat | `~/dev/bnb/platform/apps/binks-chat` | 5173 | React frontend (Vite) |

The frontend proxies `/api/*` and `/ws/*` to the backend via Vite config.

## Prerequisites

- Rust toolchain (stable)
- Node.js 18+ and pnpm 10+
- Ollama running (local or remote)

## Environment Setup

```bash
export OLLAMA_URL=http://192.168.1.4:11434  # Or http://localhost:11434
export OLLAMA_MODEL=llama3.1:8b
```

## Running the Full Stack

### Terminal 1 - Backend
```bash
cd ~/dev/home/binks-agent-orchestrator
cargo run --release --bin agent -- web
```

### Terminal 2 - Frontend
```bash
cd ~/dev/bnb/platform
pnpm dev --filter=@blacknbrownstudios/binks-chat
```

### Access
- **UI:** http://localhost:5173
- **API:** http://localhost:7317/api
- **Health:** http://localhost:7317/api/health

## Testing Changes

### Frontend
Vite hot-reloads automatically. Changes appear instantly.

### Backend
Stop with Ctrl+C, then restart. For auto-reload:
```bash
cargo install cargo-watch
cargo watch -x 'run --bin agent -- web'
```

### CLI Testing (no frontend needed)
```bash
# Simple chat
cargo run --bin agent -- chat "Hello"

# Agent with tools
cargo run --bin agent -- agent "What's my CPU usage?"

# List available tools
cargo run --bin agent -- tools

# Call specific tool
cargo run --bin agent -- call get_system_summary
```

## Troubleshooting

| Issue | Check |
|-------|-------|
| WebSocket disconnects | Is backend running? `curl localhost:7317/api/health` |
| Ollama not responding | `curl $OLLAMA_URL/api/tags` |
| MCP tools missing | Check `.mcp.json`, run `cargo run --bin agent -- tools` |
