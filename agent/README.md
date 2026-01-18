# Binks Agent CLI

Rust-based AI agent with Ollama LLM and MCP tool integration.

## Build

```bash
cd agent
cargo build --release
```

## Environment

```bash
export OLLAMA_URL=http://192.168.1.4:11434  # Or http://localhost:11434
export OLLAMA_MODEL=llama3.1:8b
```

---

## Commands

### chat

Simple LLM chat (no tools).

```bash
agent chat "What is Rust?"
```

### interactive

Interactive chat session with history.

```bash
agent interactive
```

### agent

Tool-using agent mode. The LLM decides when to call tools.

```bash
# Single message
agent agent "What's my CPU usage?"

# Interactive
agent agent

# With custom system prompt
agent agent -s "You are a DevOps expert" "Check cluster health"

# Filter to specific MCP servers (good for smaller models)
agent agent --servers sysinfo,github-gh "Get system info"
```

**Interactive commands:**
- `tools` - List available tools
- `servers` - List MCP servers
- `clear` - Clear conversation history
- `quit` - Exit

### monitor

Repository monitoring agent. Polls GitHub, writes to inbox, sends notifications.

```bash
# Run once (for cron)
agent monitor --once --repos owner/repo1,owner/repo2

# Run continuously with live output
agent monitor --repos owner/repo --interval 300

# With custom system prompt
agent monitor --repos owner/repo -s "Focus on security issues"
```

**Live monitoring view:**
```bash
# Terminal 1: Run monitor
agent monitor --repos kblack0610/binks-agent-orchestrator --interval 60

# Terminal 2: Watch inbox updates
tail -f ~/.notes/inbox/$(date +%Y-%m-%d).md
```

### tools

List available MCP tools.

```bash
# All tools
agent tools

# From specific server
agent tools --server github-gh
```

### call

Call an MCP tool directly.

```bash
agent call get_system_summary

agent call gh_issue_list --args '{"repo": "owner/repo", "state": "open"}'

agent call write_inbox --args '{"message": "Test", "source": "manual"}'
```

### serve

Run the agent as an MCP server (exposes `chat` and `agent_chat` tools).

```bash
agent serve

# With custom system prompt
agent serve -s "You are a helpful assistant"
```

---

## Configuration

MCP servers are configured in `.mcp.json` at the project root:

```json
{
  "mcpServers": {
    "sysinfo": {
      "command": "./mcps/sysinfo-mcp/target/release/sysinfo-mcp"
    },
    "github-gh": {
      "command": "./mcps/github-gh/target/release/github-gh-mcp"
    }
  }
}
```

---

## Examples

```bash
# Check system health
agent agent "What's my disk and memory usage?"

# GitHub workflow
agent agent "List open PRs in kblack0610/my-repo"

# Kubernetes (if configured)
agent agent --servers kubernetes "List all pods in the default namespace"

# Monitor repos continuously
agent monitor --repos myorg/repo1,myorg/repo2 --interval 600
```
