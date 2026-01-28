# Binks Global AI - Roadmap

## Vision

Build a **Global AI** that manages your entire digital life - not just code, but infrastructure, home automation, research, and more.

---

## Current Capabilities (January 2026)

The Rust-based agent is operational with a full MCP tool ecosystem:

```
┌─────────────────────────────────────────────────────────┐
│                    Binks Agent (Rust)                     │
├─────────────────────────────────────────────────────────┤
│  CLI Modes            │  Infrastructure                  │
│  ─────────            │  ──────────────                  │
│  chat (simple LLM)    │  Ollama (local LLM on M3 Ultra) │
│  interactive (REPL)   │  SQLite (persistence)            │
│  agent (tool-using)   │  Axum (web server + WebSocket)   │
│  monitor (autonomous) │  MCP (tool protocol)             │
│  serve (MCP server)   │                                  │
│  web (UI backend)     │                                  │
├─────────────────────────────────────────────────────────┤
│  MCP Servers (9 implemented + 2 external)                │
│  ┌────────────┐ ┌────────────┐ ┌──────────────┐          │
│  │ github-gh  │ │  sysinfo   │ │  filesystem  │          │
│  │ (44 tools) │ │ (10 tools) │ │  (14 tools)  │          │
│  ├────────────┤ ├────────────┤ ├──────────────┤          │
│  │   inbox    │ │   notify   │ │     exec     │          │
│  │ (5 tools)  │ │ (6 tools)  │ │  (5 tools)   │          │
│  ├────────────┤ ├────────────┤ ├──────────────┤          │
│  │  git-mcp   │ │ memory-mcp │ │  web-search  │          │
│  │ (10 tools) │ │ (12 tools) │ │  (6 tools)   │          │
│  ├────────────┤ ├────────────┤                            │
│  │ kubernetes │ │    ssh     │  (external, Node.js)       │
│  └────────────┘ └────────────┘                            │
└─────────────────────────────────────────────────────────┘
```

**112 tools** across 11 MCP servers, with structured logging, health checks, metrics, and a web UI.

---

## Development Phases

### Phase 1: Crawl ✅ COMPLETE
**Goal:** Basic agent functionality with core tools

- [x] Master Agent with Ollama integration
- [x] Custom KubectlToolkit for K8s management
- [x] Custom AgentSpawnerToolkit for worker agents
- [x] Pre-built ShellTools for code editing
- [x] Pre-built FileTools for file operations
- [x] FastAPI server for remote access
- [x] Clear error messages (fail fast philosophy)

### Phase 2: Walk 🟡 MOSTLY COMPLETE
**Goal:** Expanded capabilities and reliability

- [x] Web search (web-search-mcp with SearXNG backend)
- [x] Agent memory/persistence (SQLite + memory-mcp with dual-layer architecture)
- [x] Monitoring for repo health (monitor subcommand + inbox-mcp + notify-mcp)
- [x] Structured logging (tracing with correlation IDs)
- [ ] SQL toolkit for database queries
- [ ] Authentication to API

### Phase 3: Run
**Goal:** Multi-agent orchestration and specialization

- [x] MCP integration (11 servers, full client/server protocol)
- [ ] Specialized worker agents:
  - Code Review Agent
  - Security Audit Agent
  - Research Agent
  - Home Automation Agent
- [ ] Agent-to-agent communication
- [ ] Task queue and scheduling

### Phase 4: Fly
**Goal:** Full Global AI autonomy

- [ ] Proactive monitoring and auto-remediation
- [ ] Learning from past interactions
- [ ] Cross-domain orchestration (code + infra + home)
- [ ] Voice/natural interface options
- [ ] Mobile notifications and control

---

## Stability & Code Quality Phases

### Phase S1: Agent Stability (P0) ✅ COMPLETE
**Goal:** Make binks reliable for production use
**Philosophy:** Fail fast, fail loud (see ARCHITECTURE.md)

- [x] Add configurable timeouts (LLM: 5min default, tools: 1min default)
- [x] Make MAX_ITERATIONS configurable (default: 10)
- [x] Add conversation history pruning (max_history_messages: 100)
- [x] Clear error messages on failure

> **Note:** Retry logic and model fallback chains are explicitly **not** implemented.
> Per ARCHITECTURE.md, these are anti-patterns that hide failures and make debugging harder.

### Phase S2: Observability (P1) ✅ COMPLETE
**Goal:** Debug failures and monitor performance

- [x] Structured logging with correlation IDs (`agent/src/web/mod.rs`)
- [x] Health check HTTP endpoint (`/api/health` — checks config, MCP, LLM, tools)
- [x] Basic metrics — latency, errors, success rates (`agent/src/agent/metrics.rs`)

### Phase S3: Code Cleanup (P1) 🟢 MOSTLY COMPLETE
**Goal:** Reduce maintenance burden

- [x] Create `mcp-common` crate (init, errors, results) — used by all 9 MCPs
- [x] Consolidate workspace dependencies (`[workspace.dependencies]` in root Cargo.toml)
- [ ] Extract common parameter types (partially done — types still distributed per-MCP)
- [x] Standardize error handling across MCPs (`mcp-common/src/error.rs`)

### Phase S4: PR Testing (P2)
**Goal:** Enable automated PR review and testing

- [x] Create `exec-mcp` for command execution
- [ ] Add `test-pr` workflow
- [ ] Language-agnostic test runners (Node, Rust, Python, Go)

---

## MCP (Model Context Protocol) Strategy

MCP is the standardized tool interface for the agent. All tools are implemented as MCP servers.

### Current Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    MCP Server Hub                        │
├─────────────────────────────────────────────────────────┤
│  Tier 1 (Core)       │  Tier 2 (Common)                │
│  ──────────           │  ──────────────                 │
│  sysinfo-mcp          │  github-gh                      │
│  filesystem-mcp       │  inbox-mcp                      │
│                       │  notify-mcp                     │
│                       │  exec-mcp                       │
├───────────────────────┼─────────────────────────────────┤
│  Tier 3 (Optional)    │  Tier 4 (Meta)                  │
│  ────────────────     │  ────────────                   │
│  kubernetes (Node)    │  agent (self-hosting)           │
│  ssh (Node)           │                                 │
│  web-search-mcp       │                                 │
├───────────────────────┴─────────────────────────────────┤
│  Workspace-only (not in .mcp.json)                       │
│  git-mcp, memory-mcp                                     │
├─────────────────────────────────────────────────────────┤
│  Planned (skeleton only)                                 │
│  web-fetch-mcp, scratchpad-mcp, semantic-mcp             │
└─────────────────────────────────────────────────────────┘
```

### Future MCP Integrations

```
┌─────────────────────────────────────────────────────────┐
│  Community MCP Servers    │  Custom MCP Servers         │
│  ─────────────────────    │  ──────────────────         │
│  Spotify Control          │  Home Lab Control           │
│  Smart Home (Home Asst.)  │  Pi Cluster Metrics         │
│  Calendar Management      │  Custom App APIs            │
└─────────────────────────────────────────────────────────┘
```

### Why MCP Matters

1. **Standardization** - One protocol, many tools
2. **Community** - Leverage others' work
3. **Portability** - Switch between agents without rewriting tools

---

## Architecture Decision Records

### ADR-001: Rust + MCP Architecture
**Decision:** Rewrite from Python/Agno to Rust with native MCP support
**Rationale:**
- Single binary, no runtime dependencies
- Async-first with Tokio
- MCP is the standard protocol for tool interoperability
- Resource-efficient (4.5-13 MB binary depending on features)

### ADR-002: Modular MCP Servers
**Decision:** Each capability is a standalone MCP server
**Rationale:**
- Independent development and deployment
- Shared infrastructure via `mcp-common` crate
- Can be used by any MCP-compatible agent (Claude Code, etc.)
- Clear boundaries between capabilities

### ADR-003: Ollama for LLM
**Decision:** Use Ollama for local LLM hosting
**Rationale:**
- Privacy (no data leaves your network)
- Cost (no API fees)
- Control (choose your model)
- Performance (405B on M3 Ultra is viable)

---

## Contributing

When adding new capabilities:

1. **Build as an MCP server** - Each tool gets its own crate under `mcps/`
2. **Use `mcp-common`** - Shared init, errors, and result formatting
3. **Document in this roadmap** - Add to appropriate phase
4. **Follow fail-fast** - No retries, no fallbacks (see ARCHITECTURE.md)

---

## Legacy Archive

<details>
<summary>Python-era sections (pre-Rust rewrite)</summary>

### Agno Pre-built Toolkits (Python)

These were relevant when the agent used the Agno Python framework. The Rust rewrite replaced all of these with native MCP servers.

| Toolkit | Replaced By |
|---------|-------------|
| `ShellTools` | exec-mcp |
| `FileTools` | filesystem-mcp |
| `DuckDuckGo` | web-search-mcp (SearXNG) |
| `SlackToolkit` | notify-mcp |
| `GithubToolkit` | github-gh |

### CLI Orchestrator - Backend Providers (Python)

The Python-era orchestrator supported multiple LLM backends (Groq, OpenRouter, Gemini, Claude). The Rust agent uses Ollama exclusively for local inference.

### Active Service Projects (Python)

1. **Email Scoring LLM** - Score and categorize job application emails
2. **JobScan AI Integration** - ATS scoring for LinkedIn job descriptions

These were Python services, not part of the Rust agent.

</details>

---

## References

- [MCP Specification](https://modelcontextprotocol.io)
- [ARCHITECTURE.md](./ARCHITECTURE.md) - System design and fail-fast philosophy
- [mcps/ROADMAP.md](../mcps/ROADMAP.md) - MCP-specific roadmap
