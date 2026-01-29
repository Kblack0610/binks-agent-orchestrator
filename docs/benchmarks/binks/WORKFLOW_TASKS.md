# Binks Autonomous Workflow Tasks

**Date:** 2026-01-28
**Branch:** `feat/binks-workflow-tasks`
**Purpose:** Define and execute real autonomous tasks to validate Binks agent capabilities.

---

## Task Catalog

### Code Tasks (filesystem, git-mcp, exec-mcp, github-gh)

#### T-CODE-1: Explain a File
- **Prompt:** "Read `agent/src/agent/mod.rs` and explain what the Agent struct does, its key methods, and how it integrates with MCP tools."
- **Servers:** `["filesystem"]`
- **Model:** deepseek-r1:70b
- **Success criteria:** Accurate description of Agent struct, mentions MCP integration, key methods identified

#### T-CODE-2: Find All TODOs
- **Prompt:** "Search the codebase under `/home/kblack0610/dev/home/binks-agent-orchestrator/agent/src/` for TODO comments. List each with file, line, and content."
- **Servers:** `["filesystem"]`
- **Model:** deepseek-r1:70b
- **Success criteria:** Finds at least 8 of the 9 known TODOs

#### T-CODE-3: Git Status Summary
- **Prompt:** "Check the git status of the repository at `/home/kblack0610/dev/home/binks-agent-orchestrator` and summarize recent changes including the last 5 commits."
- **Servers:** `["git"]`
- **Model:** deepseek-r1:70b
- **Success criteria:** Returns current branch, clean/dirty status, last 5 commit messages

#### T-CODE-4: Run Clippy and Report
- **Prompt:** "Run `cargo clippy --workspace` in `/home/kblack0610/dev/home/binks-agent-orchestrator` and report any warnings or errors."
- **Servers:** `["exec"]`
- **Model:** deepseek-r1:70b
- **Success criteria:** Executes clippy, reports results (pass/fail + any warnings)

### Ops Tasks (kubernetes, ssh, sysinfo)

#### T-OPS-1: Pod Health Check
- **Prompt:** "List all pods across all namespaces in the Kubernetes cluster and report any that are not in Running/Completed state."
- **Servers:** `["kubernetes"]`
- **Model:** llama3.1:70b
- **Success criteria:** Lists pods, identifies unhealthy ones

#### T-OPS-2: Node Resource Usage
- **Prompt:** "Check the resource usage (CPU and memory) for all nodes in the Kubernetes cluster and flag any over 80% utilization."
- **Servers:** `["kubernetes"]`
- **Model:** llama3.1:70b
- **Success criteria:** Reports node metrics, identifies high-utilization nodes

#### T-OPS-3: System Health
- **Prompt:** "Get the current system memory, CPU usage, and disk space. Report any resource concerns."
- **Servers:** `["sysinfo"]`
- **Model:** llama3.1:70b
- **Success criteria:** Returns memory/CPU/disk stats, identifies any issues

### Research Tasks (web-search, memory)

#### T-RESEARCH-1: Search and Summarize
- **Prompt:** "Search for 'Rust MCP protocol specification 2025' and summarize the top 3 results."
- **Servers:** `["web-search"]`
- **Model:** deepseek-r1:70b
- **Success criteria:** Returns search results, provides coherent summary

#### T-RESEARCH-2: Fetch and Analyze
- **Prompt:** "Fetch the page at https://modelcontextprotocol.io and summarize what MCP is, its key features, and supported transports."
- **Servers:** `["web-search"]`
- **Model:** deepseek-r1:70b
- **Success criteria:** Successfully fetches page, accurate summary of MCP spec

### Communication Tasks (notify, inbox)

#### T-COMM-1: Write Inbox Note
- **Prompt:** "Write a note to the inbox with source 'benchmark', priority 'normal', tags ['tier3', 'test'], message: 'Tier 3 workflow task catalog execution complete. All tasks validated.'"
- **Servers:** `["inbox"]`
- **Model:** qwen3-coder:30b
- **Success criteria:** Message written to inbox with correct metadata

#### T-COMM-2: Read Inbox
- **Prompt:** "Read today's inbox messages and summarize them."
- **Servers:** `["inbox"]`
- **Model:** qwen3-coder:30b
- **Success criteria:** Returns inbox contents, provides summary

---

## Execution Protocol

1. Each task is run via `agent_chat` with specified servers and model
2. Record: pass/fail, tool calls made, execution time, output quality
3. Results documented in `WORKFLOW_RESULTS.md`

## Expected Limitations

Based on Tier 2B benchmark findings:
- Models may waste iterations on filesystem navigation (mitigated by server filtering)
- Models won't use memory tools unless explicitly told
- Some tool calls may fail on first attempt (parameter format issues)
- 10-iteration max may be insufficient for complex multi-step tasks
