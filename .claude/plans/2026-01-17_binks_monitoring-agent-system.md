# Monitoring Agent System Plan

## Overview

Build a two-agent architecture for autonomous repo monitoring, analysis, and reporting:

1. **Monitor Agent** - Lightweight daemon that polls, observes, and dispatches
2. **Task Agent** - Heavy worker that executes benchmarks, tests, analysis, code generation

```
┌─────────────────────────────────────────────────────────────┐
│                    Cron / systemd timer                     │
│                    (every 15-30 min)                        │
└────────────────────────────┬────────────────────────────────┘
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                     MONITOR AGENT                           │
│  • Poll GitHub for issues/PRs needing attention             │
│  • Check repo health (test status, coverage, benchmarks)    │
│  • Dispatch tasks to Task Agent                             │
│  • Write reports to ~/.notes/inbox                          │
│  • Send notifications (Slack, Discord)                      │
└─────────────────────────────┬───────────────────────────────┘
                              │ spawns when work needed
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      TASK AGENT                             │
│  • Run benchmarks and analysis                              │
│  • Generate missing E2E tests                               │
│  • Create PRs for fixes/improvements                        │
│  • Respond to review comments                               │
│  • (Later: Implement feature tickets from Linear)           │
└─────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Inbox MCP Server

**Goal**: Create a simple MCP server for local file-based inbox (`~/.notes/inbox`)

### Files to Create

```
mcps/inbox-mcp/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── server.rs
    └── types.rs
```

### Tools

| Tool | Description |
|------|-------------|
| `write_inbox` | Write a message to inbox with timestamp, priority, tags |
| `read_inbox` | Read recent inbox messages (with filters) |
| `clear_inbox` | Archive/clear old messages |

### Message Format (`~/.notes/inbox/YYYY-MM-DD.md`)

```markdown
## 2026-01-17 14:30:00 [monitor] #pr #review
PR #45 needs your review: "Fix auth bug"
https://github.com/user/repo/pull/45

---

## 2026-01-17 15:00:00 [task] #test #completed
Generated 3 new E2E tests for auth module
Files: tests/e2e/auth_test.rs
```

### Dependencies

```toml
rmcp = { version = "0.13", features = ["server", "macros", "transport-io"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1.2"
chrono = "0.4"
```

---

## Phase 2: Notify MCP Server

**Goal**: Send notifications to Slack and Discord

### Files to Create

```
mcps/notify-mcp/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── server.rs
    ├── slack.rs
    └── discord.rs
```

### Tools

| Tool | Description |
|------|-------------|
| `send_slack` | Send message via Slack webhook |
| `send_discord` | Send message via Discord webhook |
| `send_digest` | Send formatted daily digest to all channels |

### Configuration (env vars)

```bash
SLACK_WEBHOOK_URL=https://hooks.slack.com/...
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
```

---

## Phase 3: Monitor Subcommand

**Goal**: Add `agent monitor` CLI command

### Files to Modify

- `agent/src/main.rs` - Add Monitor subcommand

### Files to Create

```
agent/src/monitor/
├── mod.rs          # Monitor struct and main loop
├── poller.rs       # GitHub polling logic
└── dispatcher.rs   # Task dispatch logic
```

### CLI

```bash
# Run once (for cron)
agent monitor --once

# Run continuously with interval
agent monitor --interval 900

# Specify repos
agent monitor --repos user/repo1,user/repo2

# With digest at specific time
agent monitor --digest-time 09:00
```

### Monitor Logic

```rust
pub struct Monitor {
    agent: Agent,
    repos: Vec<String>,
}

impl Monitor {
    pub async fn run_once(&mut self) -> Result<()> {
        // 1. Poll GitHub for actionable items
        let items = self.poll_github().await?;

        // 2. Check each repo's health
        for repo in &self.repos {
            self.check_repo_health(repo).await?;
        }

        // 3. Dispatch tasks if needed
        for item in items.needs_action() {
            self.dispatch_task(&item).await?;
        }

        // 4. Write summary to inbox
        self.write_summary_to_inbox().await?;

        Ok(())
    }
}
```

---

## Phase 4: Extend github-gh MCP

**Goal**: Add tools for repo analysis and PR review

### Files to Modify

- `mcps/github-gh/src/server.rs` - Add new tools

### New Tools to Add

| Tool | Description |
|------|-------------|
| `gh_get_workflow_status` | Get CI status for a branch/PR |
| `gh_list_review_requests` | List PRs where agent is reviewer |
| `gh_get_pr_diff` | Get diff content for analysis |
| `gh_get_check_runs` | Get test/lint results |

---

## Phase 5: Task Agent Enhancements

**Goal**: Enable Task Agent to run benchmarks, generate tests

### System Prompts

```rust
const BENCHMARK_ANALYSIS_PROMPT: &str = r#"
Analyze this repository for performance. Run benchmarks if available.
Report: current metrics, regressions, and recommendations.
"#;

const TEST_GENERATION_PROMPT: &str = r#"
Analyze this codebase for missing E2E test coverage.
Generate tests for uncovered critical paths.
Follow existing test patterns and conventions.
"#;

const CODE_REVIEW_PROMPT: &str = r#"
Review this PR for: correctness, security, performance, style.
Provide actionable feedback. Be constructive.
"#;
```

### Task Types

```rust
pub enum TaskType {
    RunBenchmarks { repo: String },
    AnalyzeCoverage { repo: String },
    GenerateTests { repo: String, target: String },
    ReviewPR { repo: String, pr_number: u32 },
    ImplementTicket { ticket_id: String },  // Phase 6
}
```

---

## Phase 6: Linear Integration (Later)

**Goal**: Connect to Linear for feature ticket tracking

### Approach

Use existing Linear MCP server available in Claude's tools:
- `mcp__linear__list_issues`
- `mcp__linear__get_issue`
- `mcp__linear__update_issue`
- `mcp__linear__create_comment`

### Workflow

```
Linear Ticket → Monitor detects → Task Agent implements → PR created → Ticket updated
```

---

## Configuration

### Updated `.mcp.json`

```json
{
  "mcpServers": {
    "sysinfo": { ... },
    "github-gh": { ... },
    "kubernetes": { ... },
    "ssh": { ... },
    "inbox": {
      "command": "./mcps/inbox-mcp/target/release/inbox-mcp",
      "env": {
        "INBOX_PATH": "${HOME}/.notes/inbox",
        "RUST_LOG": "info"
      }
    },
    "notify": {
      "command": "./mcps/notify-mcp/target/release/notify-mcp",
      "env": {
        "SLACK_WEBHOOK_URL": "${SLACK_WEBHOOK_URL}",
        "DISCORD_WEBHOOK_URL": "${DISCORD_WEBHOOK_URL}",
        "RUST_LOG": "info"
      }
    }
  }
}
```

### Cron Setup

```bash
# /etc/cron.d/binks-monitor or user crontab
# Run every 15 minutes
*/15 * * * * /path/to/agent monitor --once --repos kblack0610/binks-agent-orchestrator

# Daily digest at 9am
0 9 * * * /path/to/agent monitor --digest-only
```

---

## Implementation Order

| Phase | What | Priority |
|-------|------|----------|
| 1 | Inbox MCP Server | HIGH - Foundation for all reporting |
| 2 | Notify MCP Server | HIGH - Slack/Discord alerts |
| 3 | Monitor Subcommand | HIGH - Core orchestration |
| 4 | Extend github-gh | MEDIUM - Better repo analysis |
| 5 | Task Agent Prompts | MEDIUM - Benchmark/test generation |
| 6 | Linear Integration | LOW - After basics work |

---

## Critical Files

### To Create
- `mcps/inbox-mcp/src/main.rs`
- `mcps/inbox-mcp/src/server.rs`
- `mcps/notify-mcp/src/main.rs`
- `mcps/notify-mcp/src/server.rs`
- `agent/src/monitor/mod.rs`

### To Modify
- `agent/src/main.rs` - Add Monitor subcommand
- `mcps/github-gh/src/server.rs` - Add analysis tools
- `.mcp.json` - Register new MCP servers

---

## Verification

### Phase 1 Test
```bash
# Build and test inbox MCP
cd mcps/inbox-mcp && cargo build --release
echo '{"method":"tools/list"}' | ./target/release/inbox-mcp

# Test writing
agent call write_inbox --args '{"message": "Test", "priority": "normal", "tags": ["test"]}'
cat ~/.notes/inbox/$(date +%Y-%m-%d).md
```

### Phase 3 Test
```bash
# Run monitor once
agent monitor --once --repos kblack0610/binks-agent-orchestrator

# Check inbox for results
cat ~/.notes/inbox/$(date +%Y-%m-%d).md
```

### End-to-End Test
```bash
# Create test issue in repo
gh issue create --title "Test: Add missing E2E test" --body "..."

# Run monitor
agent monitor --once

# Verify: inbox has entry, notification sent, PR created
```
