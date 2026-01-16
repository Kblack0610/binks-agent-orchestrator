# Project Cleanup: Remove Python, Add Historical Docs

**Goal:** Clean up the repo by removing all Python-related code while preserving the original orchestration vision in documentation.

## Files/Directories to Remove

### Python Code
- [ ] `orchestrator/` - Python agno-based orchestrators (entire directory)
- [ ] `client/` - Python client (entire directory)
- [ ] `services/` - email_scorer, jobscan (entire directory)
- [ ] `run_orchestrator.py` - Python entrypoint
- [ ] `test_api.py` - Python test
- [ ] `.venv/` - Python virtual environment
- [ ] `.pytest_cache/` - pytest cache
- [ ] `.orchestrator/` - orchestrator state files

### Legacy Scripts
- [ ] `quickstart.sh` - outdated setup script
- [ ] `Makefile` - Python-focused build targets

## Files to Keep

- `agent/` - Rust agent
- `mcps/` - MCP servers
- `manifests/` - K8s deployment manifests
- `scripts/` - utility scripts (review individually)
- `docs/` - update with historical context
- `.mcp.json` - MCP configuration
- `.gitignore` - update for Rust-only
- `model/` - Ollama model scripts

## New Structure to Create

```
binks-agent-orchestrator/
├── agent/                    # Rust agent (keep)
├── mcps/                     # MCP servers (keep)
│   ├── github-gh/
│   └── sysinfo-mcp/
├── test/                     # New test directory
│   └── README.md
├── manifests/                # K8s manifests (keep)
├── scripts/                  # Utility scripts (keep)
├── model/                    # Ollama scripts (keep)
├── docs/
│   ├── ARCHITECTURE.md       # Update for current state
│   ├── LEGACY_ORCHESTRATION.md  # New: historical vision
│   ├── PROJECT_STRUCTURE.md  # Update
│   └── ROADMAP.md           # Update
├── .mcp.json
├── .gitignore               # Update for Rust
└── README.md                # Update
```

## New Document: docs/LEGACY_ORCHESTRATION.md

This document will preserve the original Python orchestration vision:

### Content to Include:
1. **Original Vision** - The layered architecture (Clients → API → Agent Core)
2. **Agno Framework** - What it was and how it worked
3. **API Contract** - The REST/WebSocket endpoints that were planned
4. **Tool Architecture** - KubectlToolkit, AgentSpawnerToolkit
5. **Why We Moved On** - Evolution to Rust agent + MCP approach
6. **Lessons Learned** - What was valuable from the design

### Source Material:
- `orchestrator/README.md` - Overview of Python orchestrator
- `docs/ARCHITECTURE.md` - Detailed layered architecture
- `.orchestrator/*.md` - Context files

## Implementation Steps

1. **Create docs/LEGACY_ORCHESTRATION.md**
   - Consolidate orchestrator/README.md content
   - Preserve API contract from ARCHITECTURE.md
   - Add "Why We Evolved" section explaining shift to Rust/MCP

2. **Create test/ directory**
   - `test/README.md` - explain test structure

3. **Remove Python directories**
   ```bash
   rm -rf orchestrator/ client/ services/
   rm -rf .venv/ .pytest_cache/ .orchestrator/
   rm run_orchestrator.py test_api.py
   rm quickstart.sh Makefile
   ```

4. **Update docs/ARCHITECTURE.md**
   - Reflect new Rust agent + MCP architecture
   - Remove references to Python

5. **Update README.md**
   - Reflect current project state
   - Link to LEGACY_ORCHESTRATION.md for history

6. **Update .gitignore**
   - Remove Python entries
   - Keep Rust entries (target/, etc.)

## Verification

1. Ensure no broken references remain
2. `cargo build --release` still works for agent and MCPs
3. `.mcp.json` configuration still valid
4. All docs are internally consistent
