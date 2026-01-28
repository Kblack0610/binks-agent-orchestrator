# Project Instructions

## Git Workflow

**Always commit to a new branch** — never commit directly to `master`.

- Create a feature branch before committing: `feat/<description>`, `fix/<description>`, or `chore/<description>`
- Push the branch and create a PR for review
- Follow conventional commit format: `type(scope): description`

Branch naming:
- `feat/` — new features or capabilities
- `fix/` — bug fixes
- `chore/` — maintenance, docs, dependency updates
- `refactor/` — code restructuring without behavior change

## Build & Test

```bash
# Build a specific MCP
cargo build -p <crate-name>

# Lint
cargo clippy -p <crate-name>

# Build entire workspace
cargo build --workspace
```

## Architecture

- Each capability is an MCP server under `mcps/`
- Shared code lives in `mcps/mcp-common/`
- Agent core is in `agent/`
- See `docs/ARCHITECTURE.md` and `docs/ROADMAP.md` for details
