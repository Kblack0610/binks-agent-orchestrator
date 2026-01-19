# MCP Roadmap

## Current MCPs

| MCP | Language | Status | Description |
|-----|----------|--------|-------------|
| github-gh | Rust | Production | GitHub CLI wrapper for issues, PRs, workflows |
| sysinfo-mcp | Rust | Production | Cross-platform system information |
| inbox-mcp | Rust | Production | Local file-based inbox for notifications |
| notify-mcp | Rust | Production | Slack/Discord webhook notifications |
| kubernetes | Node | External | Kubernetes cluster management |
| ssh | Node | External | SSH remote operations |

## Planned MCPs

### Phase 1: Foundation

#### memory-mcp (Rust) - Priority 1
**Purpose:** Persistent memory across agent sessions

**Design:**
- Dual-layer architecture: session (in-memory) + persistent (SQLite)
- Session layer for working memory during tasks
- Persistent layer for knowledge graph across restarts

**Tools:**
| Tool | Description |
|------|-------------|
| `think(thought)` | Record reasoning step in session |
| `remember(key, value)` | Store in working memory |
| `recall(key)` | Retrieve from working memory |
| `get_context()` | Full session context |
| `learn(entity, facts)` | Add to knowledge graph |
| `query(pattern)` | Search knowledge graph |
| `summarize_session()` | Compress session to persistent |
| `forget(entity)` | Remove from knowledge graph |

**Dependencies:** `rusqlite`, `rmcp`, `serde`, `chrono`

---

#### filesystem-mcp (Rust) - Priority 2
**Purpose:** Sandboxed file operations

**Design:**
- Allowlist-based directory access
- Path traversal prevention
- Size limits for operations
- Optional confirmation for destructive ops

**Tools:**
| Tool | Description |
|------|-------------|
| `read_file(path)` | Read file contents |
| `write_file(path, content)` | Write/create file |
| `list_dir(path, recursive?)` | List directory contents |
| `search_files(pattern, path)` | Search by pattern |
| `file_info(path)` | Get file metadata |
| `move_file(src, dst)` | Move/rename file |
| `delete_file(path)` | Delete file |

**Security:**
```toml
# Example config
[allowed_paths]
read = ["~", "/tmp"]
write = ["~/projects", "/tmp"]
max_file_size = "10MB"
confirm_delete = true
```

**Dependencies:** `tokio::fs`, `rmcp`, `serde`

---

### Phase 2: Capabilities

#### git-mcp (Rust) - Priority 3
**Purpose:** Local git operations (complements github-gh)

**Design:**
- Uses `git2` crate (libgit2 bindings)
- Repository-local operations only
- Read-heavy, minimal writes

**Tools:**
| Tool | Description |
|------|-------------|
| `git_status(repo)` | Repository status |
| `git_diff(repo, ref?)` | Diff against ref |
| `git_log(repo, limit?)` | Commit history |
| `git_blame(repo, file)` | Blame for file |
| `git_show(repo, ref)` | Show commit |
| `git_branch_list(repo)` | List branches |
| `git_stash(repo, action)` | Stash operations |

**Dependencies:** `git2`, `rmcp`, `serde`

---

#### web-fetch-mcp (Rust) - Priority 4
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

### Phase 3: Intelligence

#### scratchpad-mcp (Rust) - Priority 5
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

#### semantic-mcp (Rust) - Priority 6
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
| memory | Build | Core to agent, tight integration |
| filesystem | Build | Security-critical, need custom sandboxing |
| git | Build | git2 crate excellent, complements github-gh |
| web-fetch | Build | Simple HTTP easy, reqwest is great |
| scratchpad | Build | Simple, fits specific needs |
| semantic | Build (tree-sitter) | Lightweight layer sufficient for most tasks |
| browser | Use Playwright | Chromium too complex |
| code-exec | Use Docker | Sandboxing hard, not worth building |
| advanced-semantic | Use Serena | LSP integration mature |

---

## Implementation Timeline

```
Phase 1 (Foundation)
├── memory-mcp ..................... Enable agent persistence
└── filesystem-mcp ................. Enable file operations

Phase 2 (Capabilities)
├── git-mcp ....................... Local git operations
└── web-fetch-mcp ................. HTTP and HTML parsing

Phase 3 (Intelligence)
├── scratchpad-mcp ................ Structured reasoning
└── semantic-mcp (tree-sitter) .... Basic code understanding

Phase 4 (Integration)
├── Add Playwright MCP ............ Full browser support
└── Add Serena integration ........ Advanced code analysis
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Binks Agent Core                          │
├─────────────────────────────────────────────────────────────┤
│  Phase 1: Foundation                                         │
│  ┌──────────────┐ ┌──────────────┐                          │
│  │   memory     │ │  filesystem  │                          │
│  │  (dual-layer)│ │  (sandboxed) │                          │
│  └──────────────┘ └──────────────┘                          │
├─────────────────────────────────────────────────────────────┤
│  Phase 2: Capabilities                                       │
│  ┌──────────────┐ ┌──────────────┐                          │
│  │     git      │ │  web-fetch   │                          │
│  │   (git2)     │ │  (reqwest)   │                          │
│  └──────────────┘ └──────────────┘                          │
├─────────────────────────────────────────────────────────────┤
│  Phase 3: Intelligence                                       │
│  ┌──────────────┐ ┌──────────────┐                          │
│  │  scratchpad  │ │   semantic   │                          │
│  │  (reasoning) │ │ (tree-sitter)│                          │
│  └──────────────┘ └──────────────┘                          │
├─────────────────────────────────────────────────────────────┤
│  Existing Production MCPs                                    │
│  ┌─────────┐ ┌─────────┐ ┌───────┐ ┌────────┐              │
│  │github-gh│ │ sysinfo │ │ inbox │ │ notify │              │
│  └─────────┘ └─────────┘ └───────┘ └────────┘              │
├─────────────────────────────────────────────────────────────┤
│  External MCPs                                               │
│  ┌─────────┐ ┌─────────┐ ┌──────────┐ ┌────────┐           │
│  │   k8s   │ │   ssh   │ │playwright│ │ serena │           │
│  └─────────┘ └─────────┘ └──────────┘ └────────┘           │
└─────────────────────────────────────────────────────────────┘
```
