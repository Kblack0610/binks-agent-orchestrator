# MCP Servers

Overview of all Model Context Protocol servers in this project.

## Available Servers

| Server | Language | Tools | Description |
|--------|----------|-------|-------------|
| `github-gh` | Rust | 21 | GitHub CLI wrapper (issues, PRs, workflows, analysis) |
| `sysinfo-mcp` | Rust | 8 | System info (CPU, memory, disk, network) |
| `inbox-mcp` | Rust | 3 | Local file-based inbox for agent reports |
| `notify-mcp` | Rust | 4 | Slack/Discord notifications |
| `kubernetes` | Node | 24 | Kubernetes cluster management |
| `ssh` | Node | 7 | SSH remote commands and file transfer |

---

## github-gh

GitHub CLI wrapper using `gh`. Requires `gh auth login`.

**Tools:**
- `gh_issue_list`, `gh_issue_view`, `gh_issue_create`, `gh_issue_edit`, `gh_issue_close`, `gh_issue_comment`
- `gh_pr_list`, `gh_pr_view`, `gh_pr_create`, `gh_pr_merge`, `gh_pr_diff`, `gh_pr_checks`, `gh_pr_comment`
- `gh_search_prs`
- `gh_repo_list`, `gh_repo_view`
- `gh_workflow_list`, `gh_workflow_run`, `gh_run_list`, `gh_run_view`, `gh_run_cancel`

```bash
# Build
cd mcps/github-gh && cargo build --release
```

---

## sysinfo-mcp

Cross-platform system information.

**Tools:**
- `get_os_info` - OS name, version, kernel
- `get_cpu_info` - Model, cores, frequency
- `get_cpu_usage` - Current CPU usage %
- `get_memory_info` - RAM and swap usage
- `get_disk_info` - Partition info and space
- `get_network_interfaces` - NICs and traffic
- `get_uptime` - System uptime
- `get_system_summary` - All info combined

```bash
cd mcps/sysinfo-mcp && cargo build --release
```

---

## inbox-mcp

Local file-based inbox for agent reports. Writes to `~/.notes/inbox/`.

**Tools:**
- `write_inbox` - Write message with timestamp, source, tags
- `read_inbox` - Read recent entries (with filters)
- `clear_inbox` - Archive old entries

**Config:**
```bash
INBOX_PATH=~/.notes/inbox  # Default location
```

**Message format** (`~/.notes/inbox/2026-01-17.md`):
```markdown
## 2026-01-17 14:30:00 [monitor] #pr #review
PR #45 needs review: "Fix auth bug"
```

```bash
cd mcps/inbox-mcp && cargo build --release
```

---

## notify-mcp

Slack and Discord notifications via webhooks.

**Tools:**
- `send_slack` - Send Slack message
- `send_discord` - Send Discord message
- `send_digest` - Send to all configured channels
- `get_notify_status` - Check webhook configuration

**Config:**
```bash
export SLACK_WEBHOOK_URL=https://hooks.slack.com/...
export DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
```

```bash
cd mcps/notify-mcp && cargo build --release
```

---

## kubernetes (external)

Kubernetes cluster management via `npx kubernetes-mcp-server`.

**Tools:** Pods, deployments, services, namespaces, helm, events, logs, exec.

**Config:**
```bash
KUBECONFIG=~/.kube/config
```

---

## ssh (external)

SSH operations via `npx @aiondadotcom/mcp-ssh`.

**Tools:** `runRemoteCommand`, `uploadFile`, `downloadFile`, `listKnownHosts`, `checkConnectivity`.

Uses `~/.ssh/config` for host definitions.

---

## Adding a New MCP Server

1. Create project:
   ```bash
   mkdir mcps/my-mcp && cd mcps/my-mcp
   cargo init
   ```

2. Add dependencies to `Cargo.toml`:
   ```toml
   rmcp = { version = "0.13", features = ["server", "macros", "transport-io"] }
   ```

3. Implement with `#[tool_router]` macro:
   ```rust
   #[tool_router]
   impl MyMcpServer {
       #[tool(description = "Does something")]
       async fn my_tool(&self, params: Params) -> Result<CallToolResult, McpError> {
           // ...
       }
   }
   ```

4. Register in `.mcp.json`:
   ```json
   "my-mcp": {
     "command": "./mcps/my-mcp/target/release/my-mcp"
   }
   ```
