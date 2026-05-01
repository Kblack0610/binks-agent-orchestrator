# binks-agent-orchestrator

A collection of [Model Context Protocol](https://modelcontextprotocol.io) servers (Rust, built on [rmcp](https://crates.io/crates/rmcp)) plus shared crates, intended as the foundation for a terminal-based orchestrator that complements OpenCode for coding sessions.

## Status

The autonomous Binks Agent loop that previously lived here was extracted to a separate archived repo: see [`~/dev/home/binks/`](../binks/) (locally). What remains is the MCP toolbox and the shared layer.

## Layout

```
binks-agent-orchestrator/
├── common/mcp-common/   # Shared MCP utilities (init, errors, formatting)
├── mcps/                # MCP servers — each builds to a standalone binary
│   ├── filesystem-mcp/  # Sandboxed file ops
│   ├── git-mcp/         # libgit2-backed local git
│   ├── github-gh/       # gh CLI wrapper (issues, PRs, runs)
│   ├── exec-mcp/        # Sandboxed command execution
│   ├── web-search-mcp/  # SearXNG search + HTTP fetch
│   ├── sysinfo-mcp/     # OS/CPU/mem/disk/net info
│   ├── memory-mcp/      # Session + persistent memory layers
│   ├── task-mcp/        # Task CRUD + dependencies
│   ├── knowledge-mcp/   # Cross-repo FTS5 doc index
│   ├── inbox-mcp/       # Local file-based agent inbox
│   ├── notify-mcp/      # Slack/Discord webhooks
│   ├── linear-cli-mcp/  # Linear via the linear CLI
│   ├── rico-mcp/        # RICO mobile UI dataset search
│   ├── adb-mcp/         # Android Debug Bridge
│   ├── sql-mcp/         # SQL queries (read-only by default)
│   └── unity-mcp/       # Unity Editor log monitoring
├── apps/pick-a-number/  # (placeholder)
├── docker/, manifests/  # Container + k8s manifests
├── scripts/             # Install + utility scripts
├── docs/                # Architecture and design notes
└── .mcp.json            # MCP server registration for Claude Code
```

## Build

```bash
# Whole workspace
cargo build --workspace --release

# A single MCP
cargo build -p filesystem-mcp --release
```

Binaries land in `target/release/`. `.mcp.json` registers them for Claude Code.

## Development

- Format: `cargo fmt`
- Lint: `cargo clippy --workspace -- -D warnings`
- Test: `cargo test --workspace`

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) and [`docs/ROADMAP.md`](docs/ROADMAP.md) for design notes.

## License

MIT
