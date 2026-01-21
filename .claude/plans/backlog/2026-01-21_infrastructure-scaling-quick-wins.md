# Infrastructure Scaling Assessment: binks-agent-orchestrator

**Status:** Backlog
**Created:** 2026-01-21
**Priority:** Medium (Quick wins available)

---

## Summary

**Overall verdict:** Infrastructure is **well-organized** but has **scaling bottlenecks**. Architecture is clean and modular, but optimized for single-user, sequential execution.

---

## Current State: What's Working Well

| Area | Status | Notes |
|------|--------|-------|
| Directory structure | ✓ Excellent | Clean separation: agent/, orchestrator/, mcps/, common/ |
| MCP server modularity | ✓ Excellent | 8 independent servers, each self-contained |
| Documentation | ✓ Good | ARCHITECTURE.md, PROJECT_STRUCTURE.md exist |
| Cargo workspace | ✓ Good | Well-defined members, shared deps |
| MCP daemon | ✓ Good | Connection pooling architecture exists |
| Prompt organization | ✓ Good | Specialized agents with clear roles |

---

## Scaling Concerns by Area

### 1. Agent Internals (`agent/src/`)

| Issue | Severity | Impact |
|-------|----------|--------|
| Sequential tool execution | High | Cannot parallelize independent tool calls |
| Single SQLite connection mutex | High | All DB ops serialize through one lock |
| Blocking stdin in REPL | Medium | Blocks async runtime during input |
| Tool cache has no TTL | Medium | Stale tool lists if servers change |
| Unbounded event channel | Low | Potential memory growth |

**Key files:**
- `agent/src/agent/mod.rs` - Tool execution loop (line 85: MAX_ITERATIONS=10)
- `agent/src/db/mod.rs` - Single `Arc<Mutex<Connection>>`
- `agent/src/mcp/client.rs` - Cache without invalidation

### 2. Orchestrator (`orchestrator/src/`)

| Issue | Severity | Impact |
|-------|----------|--------|
| No parallel workflow execution | High | One workflow at a time |
| New Agent/MCP pool per step | High | Repeated connection overhead |
| No workflow state persistence | High | Cannot recover from crashes |
| Parallel/Branch steps unimplemented | Medium | Limited workflow expressiveness |
| Checkpoints are CLI-only, blocking | Medium | Cannot integrate external approvals |

**Key files:**
- `orchestrator/src/engine.rs` - Line 193: creates new `McpClientPool` per step
- `orchestrator/src/workflow.rs` - `Parallel` has `#[serde(skip)]`
- `orchestrator/src/checkpoint.rs` - Synchronous stdin blocking

### 3. Overall Architecture

| Issue | Severity | Impact |
|-------|----------|--------|
| Hardcoded absolute paths in `.mcp.json` | High | Not portable across machines |
| No config inheritance/layering | Medium | Duplicate configs, no env-specific overrides |
| No startup config validation | Medium | Errors only at runtime |
| Duplicate config loading logic | Low | agent/ and orchestrator/ both have loaders |
| No common MCP error types | Low | Inconsistent error handling |

**Key files:**
- `.mcp.json` - Contains absolute paths
- `agent/src/config.rs` - No validation
- `common/mcp-common/src/lib.rs` - Missing unified types

---

## Recommended Improvements

### Tier 1: High Priority (Unlock Scaling)

1. **Share McpClientPool across workflow steps**
   - File: `orchestrator/src/engine.rs:193`
   - Change: Pass pool reference through workflow execution

2. **Add parallel tool execution**
   - File: `agent/src/agent/mod.rs`
   - Change: Use `tokio::join_all` for independent tools

3. **Replace absolute paths with workspace-relative**
   - File: `.mcp.json`
   - Change: Use `./mcps/...` or `${WORKSPACE}/mcps/...` pattern

4. **Implement workflow state persistence**
   - Add: SQLite workflow state table
   - Impact: Resume interrupted workflows

### Tier 2: Medium Priority (Production Readiness)

5. **Async database access** - Use `tokio_rusqlite` or `sqlx`
6. **Implement Parallel/Branch workflow steps**
7. **Add config layering** - `.mcp.json` → `.mcp.local.json` → `.mcp.{env}.json`
8. **Async checkpoint handlers** - Enable Slack/webhook approvals

### Tier 3: Lower Priority (Nice to Have)

9. Create unified MCP error types in `mcp-common`
10. Extract tool parser to standalone crate
11. Add command lookup HashMap (O(1) vs O(n))
12. Tool cache TTL/invalidation

---

## Quick Wins (Start Here)

### Task 1: Fix Absolute Paths in `.mcp.json`
- Replace `/home/kblack0610/dev/...` with `./mcps/...`
- Test MCP servers still start

### Task 2: Add Tool Cache TTL
- File: `agent/src/mcp/client.rs`
- Add `Instant` timestamp, refresh if >5 min old

### Task 3: Convert Command Registry to HashMap
- File: `agent/src/cli/commands/mod.rs`
- O(1) lookup instead of O(n)

---

## Verification

After implementing:
1. `cargo build --release`
2. Start agent: `./target/release/binks-agent`
3. Test MCP tools load correctly
4. Run a simple workflow
