# Architecture

This repo provides a set of standalone Rust [MCP](https://modelcontextprotocol.io) servers that future orchestrator work (terminal-based, complementing OpenCode) will build on.

## Layers

```
┌────────────────────────────────────────────────────────────┐
│  Orchestrator (TBD — terminal-based, OpenCode companion)   │
└────────────────────────────────────────────────────────────┘
                            │ MCP protocol
                            ▼
┌────────────────────────────────────────────────────────────┐
│  MCP servers (mcps/*)                                      │
│  filesystem, git, github-gh, exec, web-search, sysinfo,    │
│  memory, task, knowledge, inbox, notify, linear-cli,       │
│  rico, adb, sql, unity                                     │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
                ┌──────────────────────┐
                │  common/mcp-common   │
                │  shared init, errors │
                └──────────────────────┘
```

## Conventions

- Each MCP is a standalone binary built with [`rmcp`](https://crates.io/crates/rmcp)'s `#[tool_router]` macro.
- Servers are sandboxed where appropriate (`filesystem-mcp`, `exec-mcp` use allow/deny lists from a TOML config).
- Stateless tool servers; persistent state lives in `~/.binks/` (e.g. `task-mcp` SQLite at `~/.binks/conversations.db`).
- No hidden recovery: timeouts and clear errors over retries / fallback chains.

## Adding a new MCP

1. `cargo new --lib mcps/<name>-mcp` and add to `[workspace]` members in root `Cargo.toml`.
2. Use `mcp-common` for server init and shared error types where it fits.
3. Register the binary in `.mcp.json`.

## History

The autonomous Binks Agent loop and its dependent tooling (`agent/`, `binks-bench/`, `binks-ffi/`, `self-healing-mcp/`, `workflow-mcp/`, `agent-registry-mcp/`, `agent-panel/`) was extracted to a separate archived repo at `~/dev/home/binks/` on 2026-04-30. The legacy architecture and roadmap docs from when this repo housed the agent are preserved there under `docs/legacy/`.
