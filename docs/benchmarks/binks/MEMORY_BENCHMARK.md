# Memory-Enhanced Agent Benchmark

**Date:** 2026-01-28
**Branch:** `feat/benchmark-memory-enhanced`
**PR:** Tier 2B of roadmap plan

---

## Objective

Determine whether enabling `memory-mcp` tools (think, remember, recall, learn, query, etc.)
improves the quality and structure of complex analytical tasks performed by the Binks agent.

## Methodology

### Task
Analyze 9 real TODO comments from the binks-agent-orchestrator Rust codebase:

| # | Location | TODO |
|---|----------|------|
| 1 | `agent/src/web/workflows.rs:271` | Implement async execution with checkpoint polling |
| 2 | `agent/src/web/workflows.rs:320` | Implement background execution with checkpoint handling |
| 3 | `agent/src/web/workflows.rs:336` | Look up run from storage |
| 4 | `agent/src/web/workflows.rs:350` | Look up run and submit checkpoint response |
| 5 | `agent/src/web/ws.rs:237` | (priority) Implement agent task cancellation |
| 6 | `agent/src/web/ws.rs:344` | Extract tool_calls from agent |
| 7 | `agent/src/orchestrator/engine.rs:409` | (future) Parallel agent execution |
| 8 | `agent/src/orchestrator/engine.rs:424` | (future) Conditional branching |
| 9 | `apps/agent-panel/src/ipc.rs:13` | Implement IPC server in subscription |

Each model was asked to categorize by Category, Priority (P0-P3), and Component,
then summarize the most important work areas.

### Models Tested
- **qwen3-coder:30b** — Default coding model
- **deepseek-r1:70b** — Reasoning-focused model
- **gpt-oss:120b** — Large general-purpose model

### Experiments

| Experiment | Servers | Prompt | Purpose |
|-----------|---------|--------|---------|
| **A** | `["memory"]` | Explicit: "use the `think` tool" | Test directed memory usage |
| **B** | `["memory"]` | No mention of tools | Test spontaneous memory usage |
| **C** | *(no filter — all tools)* | No mention of tools | Baseline with full toolset |

---

## Results

### Experiment A: Memory Tools + Explicit Prompting

All models used the `think` tool when explicitly instructed.

| Model | Run ID | Iterations | Tool Calls | Time | Used Think? | Output Format |
|-------|--------|-----------|------------|------|-------------|---------------|
| qwen3-coder:30b | `8e11bdb2` | 1 | 0* | 7s | **Yes** | Structured JSON |
| deepseek-r1:70b | `9f906849` | 1 | 0* | 8s | **Yes** | Structured JSON |
| gpt-oss:120b | `58d8be2d` | 1 | 0* | 10s | **Yes** | Structured JSON |

*\*Tool calls show 0 because `think` is an inline tool — the content appears in the response body, not as a separate tool invocation.*

**Output quality:** All three models produced nearly identical structured JSON with:
- 6 categories (async execution, data extraction, orchestration, storage integration, task management, IPC)
- Consistent priority assignments (P0: task cancellation, P1: async execution, P2: data persistence/orchestration, P3: IPC)
- Component grouping by file path
- gpt-oss:120b added `priority_analysis` section with per-item reasoning

### Experiment B: Memory Tools Available, No Explicit Prompting

No model used memory tools when not explicitly instructed.

| Model | Run ID | Iterations | Tool Calls | Time | Used Think? | Output Format |
|-------|--------|-----------|------------|------|-------------|---------------|
| qwen3-coder:30b | `dd6a4f27` | 1 | 0 | 5s | **No** | Markdown prose |
| deepseek-r1:70b | `a9812cd7` | 1 | 0 | 6s | **No** | Markdown table + prose |
| gpt-oss:120b | `7d7fb21a` | 1 | 0 | 5s | **No** | Markdown prose |

**Output quality:** Similar analytical quality to Experiment A, but:
- Output is in direct markdown format (not JSON wrapped in `think`)
- Slightly faster (5-6s vs 7-10s) — no overhead from tool formatting
- deepseek-r1:70b produced a table format; others used bulleted lists
- Priority assignments were consistent with Experiment A

### Experiment C: All Tools (No Server Filter)

Models defaulted to filesystem exploration instead of direct analysis.

| Model | Run ID | Iterations | Tool Calls | Time | Hit Max? | Behavior |
|-------|--------|-----------|------------|------|----------|----------|
| qwen3-coder:30b | `7716b3ba` | 10 | 10 | 2m 39s | **YES** | Navigated filesystem tree, never answered |
| deepseek-r1:70b | `4bc1e5ff` | 2 | 1 | 18s | No | Tried `run_command` (failed), then answered directly |
| gpt-oss:120b | `08afd8cb` | 10 | 9 | 1m 8s | **YES** | Read actual source files, then produced analysis |

**Observations:**
- **qwen3-coder:30b** wasted all 10 iterations on `list_dir` calls navigating the filesystem
- **deepseek-r1:70b** recovered quickly — tried one exec call, failed, then answered from prompt content
- **gpt-oss:120b** actually read the referenced source files (3x `read_file`, 4x `list_dir`), producing the most context-aware analysis but at 10x the cost

---

## Analysis

### Key Findings

#### 1. Models Never Spontaneously Use Memory Tools
When memory tools are available but not explicitly requested, models ignore them entirely.
This is consistent across all three models tested. The `think` tool provides structured
reasoning space, but models default to inline reasoning unless directed.

#### 2. Server Filtering Is Critical for Performance
| Configuration | Avg Time | Avg Iterations | Success Rate |
|--------------|----------|---------------|--------------|
| `servers: ["memory"]` + explicit | 8.3s | 1.0 | 100% |
| `servers: ["memory"]` + implicit | 5.3s | 1.0 | 100% |
| No filter (all tools) | 1m 22s | 7.3 | 33% (1/3 completed) |

Filtering to relevant servers prevents models from wasting iterations on filesystem
exploration when the information is already in the prompt.

#### 3. Analysis Quality Is Prompt-Dependent, Not Tool-Dependent
The analytical quality (categorization accuracy, priority reasoning, component grouping)
was equivalent across Experiments A and B. The `think` tool changed output format
(JSON vs markdown) but not analytical depth.

#### 4. deepseek-r1:70b Shows Best Recovery Behavior
When given all tools and encountering errors, deepseek-r1:70b recovered in 2 iterations
(18s) while both other models consumed all 10 iterations. This suggests stronger
self-correction and prompt-following when tools fail.

### Recommendations

1. **Always use `servers` parameter** when calling `agent_chat` — filter to the minimum
   required server set for the task
2. **Do not rely on models spontaneously using memory tools** — if you want structured
   thinking via `think`, you must explicitly instruct the model
3. **Memory tools are best for multi-turn sessions** — the `think` tool adds value when
   building up analysis across multiple interactions (session persistence), not for
   single-shot analysis tasks
4. **Consider system prompts** that instruct models to use `think` for complex analysis tasks
5. **deepseek-r1:70b** is the most resilient model when tool availability is unpredictable

---

## Raw Run IDs

For detailed trace analysis, use `get_run` with these IDs:

| Experiment | qwen3-coder:30b | deepseek-r1:70b | gpt-oss:120b |
|-----------|----------------|-----------------|--------------|
| A (explicit) | `8e11bdb2` | `9f906849` | `58d8be2d` |
| B (implicit) | `dd6a4f27` | `a9812cd7` | `7d7fb21a` |
| C (all tools) | `7716b3ba` | `4bc1e5ff` | `08afd8cb` |
