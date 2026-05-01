# binks-agent-orchestrator instructions

This repo houses Rust MCP servers and shared crates. The autonomous Binks Agent that used to live here has been extracted to `~/dev/home/binks/` (archived).

## Scope

- All work happens within this repo: MCP servers under `mcps/`, the shared `common/mcp-common/`, scripts, manifests, docs.
- There is no separate frontend worktree associated with this repo anymore — the previous `binks-chat` frontend was tied to the archived agent.

## Conventions

- Branches: `feat/`, `fix/`, `chore/`, `refactor/` — never commit to `master` directly.
- Conventional commits: `type(scope): description`.
- Verify with `cargo build --workspace` and `cargo clippy --workspace -- -D warnings`.

See [README.md](README.md) for the full layout.
