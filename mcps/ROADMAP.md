# MCP Roadmap

## Current MCPs (January 2026)

| MCP | Language | Status | Tools | In .mcp.json | Description |
|-----|----------|--------|-------|--------------|-------------|
| github-gh | Rust | Production | 44 | Yes (tier 2) | GitHub CLI wrapper for issues, PRs, workflows |
| sysinfo-mcp | Rust | Production | 10 | Yes (tier 1) | Cross-platform system information |
| filesystem-mcp | Rust | Production | 14 | Yes (tier 1) | Sandboxed file ops, batch reads, atomic writes |
| exec-mcp | Rust | Production | 5 | Yes (tier 2) | Command execution with security guards |
| inbox-mcp | Rust | Production | 5 | Yes (tier 2) | Local file-based inbox for notifications |
| notify-mcp | Rust | Production | 6 | Yes (tier 2) | Slack/Discord webhook notifications |
| web-search-mcp | Rust | Production | 6 | Yes (tier 3) | SearXNG-backed web search |
| git-mcp | Rust | Implemented | 10 | No | Local git operations via libgit2 |
| memory-mcp | Rust | Implemented | 12 | No | Dual-layer memory (session + persistent SQLite) |
| kubernetes | Node | External | — | Yes (tier 3) | Kubernetes cluster management |
| ssh | Node | External | — | Yes (tier 3) | SSH remote operations |

**Total:** 112 tools across 9 Rust MCPs + 2 external Node.js MCPs

> **Note:** git-mcp and memory-mcp are implemented and in the Cargo workspace but not yet configured in `.mcp.json`. Add them when ready for production use.

---

## Planned MCPs

Three MCPs have specification documents (README.md) but no implementation yet:

### web-fetch-mcp (Rust) - Next Priority
**Purpose:** HTTP fetching and HTML parsing

**Design:**
- Simple HTTP client wrapper
- HTML to text/markdown conversion
- CSS selector-based extraction
- Rate limiting and caching

**Tools:**
| Tool | Description |
|------|-------------|
| `fetch(url)` | Fetch raw content |
| `fetch_json(url)` | Fetch and parse JSON |
| `parse_html(url, selector)` | Extract via CSS selector |
| `fetch_markdown(url)` | Convert HTML to markdown |

**Dependencies:** `reqwest`, `scraper`, `rmcp`

---

### scratchpad-mcp (Rust)
**Purpose:** Structured reasoning and thinking chain

**Design:**
- Lightweight alternative to sequential-thinking
- Tracks reasoning steps with revision support
- Confidence scoring
- Session-scoped (clears between tasks)

**Tools:**
| Tool | Description |
|------|-------------|
| `think(thought, confidence?)` | Record reasoning step |
| `revise(step_id, new_thought)` | Revise previous step |
| `get_reasoning_chain()` | All thinking steps |
| `summarize_thinking()` | Condensed chain |
| `clear_scratchpad()` | Reset |

**Data structure:**
```rust
struct ThinkingStep {
    step_num: u32,
    thought: String,
    confidence: f32,
    revises: Option<u32>,
    timestamp: DateTime<Utc>,
}
```

---

### semantic-mcp (Rust)
**Purpose:** Code understanding and navigation

**Design options:**
1. **tree-sitter based** (lightweight, fast)
   - Multi-language parsing
   - Symbol extraction
   - Basic semantic queries

2. **Serena integration** (heavyweight, powerful)
   - Full LSP support
   - Refactoring support
   - Cross-file analysis

**Recommendation:** Build tree-sitter layer first, add Serena for complex tasks

**Tools (tree-sitter layer):**
| Tool | Description |
|------|-------------|
| `get_symbols(file)` | List symbols in file |
| `find_definition(symbol)` | Find where defined |
| `find_references(symbol)` | Find usages |
| `get_outline(file)` | File structure |
| `parse_function(file, name)` | Parse specific function |

**Dependencies:** `tree-sitter`, `tree-sitter-{lang}`, `rmcp`

---

## External MCPs (Use As-Is)

### Playwright MCP (Node)
**When to use:** JS-heavy sites, full browser automation
**Source:** Official Playwright MCP

### Docker MCP (Node)
**When to use:** Container management, code execution sandbox
**Source:** Existing docker-mcp

### Serena MCP (Python)
**When to use:** Advanced refactoring, LSP-powered analysis
**Integration:** Connect as MCP client when needed

---

## Build vs Buy Decision Matrix

| MCP | Decision | Rationale |
|-----|----------|-----------|
| memory | ✅ Built | Core to agent, tight integration |
| filesystem | ✅ Built | Security-critical, custom sandboxing |
| git | ✅ Built | git2 crate excellent, complements github-gh |
| web-search | ✅ Built | Pluggable backends, control over rate limiting |
| web-fetch | Build | Simple HTTP easy, reqwest is great |
| scratchpad | Build | Simple, fits specific needs |
| semantic | Build (tree-sitter) | Lightweight layer sufficient for most tasks |
| browser | Use Playwright | Chromium too complex |
| code-exec | Use Docker | Sandboxing hard, not worth building |
| advanced-semantic | Use Serena | LSP integration mature |

---

## Implementation Timeline

```
✅ Complete
├── github-gh ..................... GitHub CLI wrapper (44 tools)
├── sysinfo-mcp .................. System information (10 tools)
├── filesystem-mcp ............... File operations (14 tools)
├── exec-mcp ..................... Command execution (5 tools)
├── inbox-mcp .................... Notification inbox (5 tools)
├── notify-mcp ................... Slack/Discord (6 tools)
├── web-search-mcp ............... SearXNG search (6 tools)
├── git-mcp ...................... Local git ops (10 tools)
└── memory-mcp ................... Dual-layer memory (12 tools)

📋 Planned
├── web-fetch-mcp ................ HTTP and HTML parsing
├── scratchpad-mcp ............... Structured reasoning
└── semantic-mcp (tree-sitter) ... Basic code understanding

🔌 External
├── Playwright MCP ............... Full browser support
├── Docker MCP ................... Container management
└── Serena MCP ................... Advanced code analysis
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Binks Agent Core                           │
├─────────────────────────────────────────────────────────────┤
│  Production MCPs (in .mcp.json)                              │
│  ┌─────────┐ ┌─────────┐ ┌────────────┐ ┌────────┐          │
│  │github-gh│ │ sysinfo │ │ filesystem │ │  exec  │          │
│  └─────────┘ └─────────┘ └────────────┘ └────────┘          │
│  ┌─────────┐ ┌─────────┐ ┌────────────┐                     │
│  │  inbox  │ │ notify  │ │ web-search │                     │
│  └─────────┘ └─────────┘ └────────────┘                     │
├─────────────────────────────────────────────────────────────┤
│  Workspace MCPs (not in .mcp.json)                           │
│  ┌─────────┐ ┌─────────┐                                    │
│  │ git-mcp │ │ memory  │                                    │
│  └─────────┘ └─────────┘                                    │
├─────────────────────────────────────────────────────────────┤
│  Planned MCPs                                                │
│  ┌──────────┐ ┌────────────┐ ┌──────────┐                   │
│  │web-fetch │ │ scratchpad │ │ semantic │                   │
│  └──────────┘ └────────────┘ └──────────┘                   │
├─────────────────────────────────────────────────────────────┤
│  External MCPs                                               │
│  ┌─────────┐ ┌─────────┐ ┌──────────┐ ┌────────┐           │
│  │   k8s   │ │   ssh   │ │playwright│ │ serena │           │
│  └─────────┘ └─────────┘ └──────────┘ └────────┘           │
└─────────────────────────────────────────────────────────────┘
```
