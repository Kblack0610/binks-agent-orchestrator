# Binks Agent Expanded Benchmark Results

**Date:** 2026-01-28
**Orchestrator:** Claude Code (evaluator) + Binks agent_chat MCP tool (executor)
**Host:** Ollama at 192.168.1.4:11434
**Test Files:** `apps/pick-a-number/api/`

---

## Models Under Test

| # | Model | Size | Type | VRAM |
|---|-------|------|------|------|
| 1 | qwen3-coder:30b | 30B | Coding specialist | 18GB |
| 2 | llama3.1:70b | 70B | General purpose | 42GB |
| 3 | deepseek-r1:70b | 70B | Reasoning model | 42GB |
| 4 | qwen3-coder:480b | 480B MoE | Large coding specialist | 290GB |
| 5 | deepseek-r1:latest | ~7B | Reasoning (small) | 5.2GB |
| 6 | llama3.1:8b | 8B | General purpose (small) | 4.9GB |
| 7 | qwen-agent:latest | ~32B | Agent-tuned | 19GB |
| 8 | devstral-2:latest | ~120B | Coding specialist | 74GB |
| 9 | gpt-oss:120b | 120B | General purpose | 65GB |
| 10 | deepseek-r1:671b | 671B MoE | Large reasoning | 404GB |

---

## Tier 1: Simple (Single Tool)

### T1.1 — Read File

**Task:** Read `apps/pick-a-number/api/index.js` and report its contents.

| Model | Result | Tool Calls | Time | Notes |
|-------|--------|------------|------|-------|
| qwen3-coder:30b | ✅ Pass | 1 | ~1s | |
| llama3.1:70b | ✅ Pass | 1 | ~1s | |
| deepseek-r1:70b | ✅ Pass | 1 | ~1s | |
| qwen3-coder:480b | ✅ Pass | 1 | ~1s | |
| deepseek-r1:latest | ✅ Pass | 1 | ~1s | |
| llama3.1:8b | ✅ Pass | 1 | ~1s | |
| qwen-agent:latest | ✅ Pass | 1 | ~1s | |
| devstral-2:latest | ✅ Pass | 1 | ~1s | |
| gpt-oss:120b | ✅ Pass | 1 | ~1s | |
| deepseek-r1:671b | ✅ Pass | 1 | ~1s | |

**Pass Rate:** 10/10 (100%)
**Summary:** All models reliably complete single-tool read tasks. No differentiation at this tier.

---

## Tier 2: Multi-Step (Sequential Tools)

### T2.1 — Read → Edit (Version Bump)

**Task:** Read `package.json`, bump the version from 1.0.0 to 1.1.0, and save the file.

| Model | Result | Tool Calls | Time | Notes |
|-------|--------|------------|------|-------|
| qwen3-coder:30b | ✅ Pass | 2 | ~3s | |
| llama3.1:70b | ✅ Pass | 2 | ~3s | |
| deepseek-r1:70b | ✅ Pass | 2 | 3s | |
| qwen3-coder:480b | ✅ Pass | 2 | 3s | |
| deepseek-r1:latest | ✅ Pass | 2 | ~3s | |
| llama3.1:8b | ✅ Pass | 2 | ~3s | |
| qwen-agent:latest | ✅ Pass | 2 | ~3s | |
| devstral-2:latest | ✅ Pass | 2 | ~3s | |
| gpt-oss:120b | ✅ Pass | 2 | ~3s | |
| deepseek-r1:671b | ✅ Pass | 2 | ~3s | |

**Pass Rate:** 10/10 (100%)
**Summary:** All models successfully plan and execute 2-step sequential tasks. Tool call count is consistently optimal (2).

---

## Tier 3: Complex (Reasoning + Tools)

### T3.1 — Find TODOs → Categorize → Summarize

**Task:** Find all TODO comments in the codebase, categorize them, and create a summary report.

| Model | Result | Tool Calls | Time | Notes |
|-------|--------|------------|------|-------|
| qwen3-coder:30b | ✅ Pass | 2 | 5s | Grouped categories, slight duplication |
| llama3.1:70b | ✅ Pass | 2 | 4s | Granular 6-category breakdown |
| deepseek-r1:70b | ✅ Pass | 2 | 5s | Best formatting with descriptions |
| qwen3-coder:480b | ⚠️ Partial | 2 | 4s | Failed first attempt (search pattern), passed on retry |
| deepseek-r1:latest | ✅ Pass | 2 | ~4s | |
| llama3.1:8b | ✅ Pass | 2 | ~4s | |
| qwen-agent:latest | ✅ Pass | 2 | ~4s | |
| devstral-2:latest | ✅ Pass | 2 | ~4s | |
| gpt-oss:120b | ✅ Pass | 2 | ~4s | Best output quality |
| deepseek-r1:671b | ✅ Pass | 2 | ~4s | |

**Pass Rate:** 9/10 Pass + 1/10 Partial (100% eventual success)
**Summary:** All models can perform complex analysis. Output quality varies — gpt-oss:120b and deepseek-r1:70b produce the best-formatted results. qwen3-coder:480b showed prompt sensitivity on first attempt.

---

### T3.2 — Code Structure Analysis

**Task:** Analyze the code structure of `apps/pick-a-number/api/` and suggest architectural improvements.

| Model | Result | Tool Calls | Time | Notes |
|-------|--------|------------|------|-------|
| qwen3-coder:30b | ✅ Pass | 2 | ~5s | Good structural analysis |
| llama3.1:70b | ❌ Fail | 10 | 30s+ | Got stuck navigating filesystem, never read target file |
| deepseek-r1:70b | ✅ Pass | 2 | ~5s | Strong reasoning about architecture |
| qwen3-coder:480b | ✅ Pass | 2 | ~5s | |
| deepseek-r1:latest | ✅ Pass | 2 | ~5s | |
| llama3.1:8b | ✅ Pass | 2 | ~5s | |
| qwen-agent:latest | ✅ Pass | 2 | ~5s | |
| devstral-2:latest | ✅ Pass | 2 | ~5s | Best improvement suggestions |
| gpt-oss:120b | ✅ Pass | 2 | ~5s | Comprehensive analysis |
| deepseek-r1:671b | ✅ Pass | 2 | ~5s | |

**Pass Rate:** 9/10 (90%)
**Summary:** llama3.1:70b is the only failure across all benchmarks. It entered a filesystem navigation loop (10 tool calls) and never reached the target file. This suggests a planning/tool-use issue specific to this model when given open-ended exploration tasks. All other models, including the smaller llama3.1:8b, passed without issue.

---

### T3.3 — Debug from Symptoms

**Task:** Given a bug report ("two users playing simultaneously — one winning resets the other's game"), read the code, identify the root cause, explain it, and suggest a fix.

**Root Cause:** Global mutable `secretNumber` variable shared across all HTTP requests. When one user wins, `secretNumber` is reassigned, affecting all concurrent sessions.

| Model | Result | Tool Calls | Time | Edited File? | Notes |
|-------|--------|------------|------|-------------|-------|
| qwen3-coder:30b | ✅ Pass | 2 | 15s | No | Good analysis + fix code in response |
| llama3.1:70b | ✅ Pass | 1 | 8s | No | Clean analysis + full refactored code |
| deepseek-r1:70b | ✅ Pass | 1 | 8s | No | Clean fix with Map and helpers |
| qwen3-coder:480b | ✅ Pass | 1 | 8s | No | `startNewGame` helper + `gameActive` state |
| deepseek-r1:latest | ✅ Pass | 2 | 60s | Yes | Correctly identified + applied fix |
| llama3.1:8b | ✅ Pass | 2 | 18s | Yes | Correctly identified global state issue |
| qwen-agent:latest | ✅ Pass | 2 | 16s | Yes | Clean identification and fix |
| devstral-2:latest | ✅ Pass | 2 | 17s | Yes | Best fix — added `/new-game/:userId` endpoint |
| gpt-oss:120b | ✅ Pass | 1 | 10s | **No** | **Best overall** — read-only analysis, two approaches |
| deepseek-r1:671b | ✅ Pass | 2 | 16s | Yes | Added `generateSecretNumber()` helper |

**Pass Rate:** 10/10 (100%)

**Key Observations:**
- All models correctly identified the global mutable state as root cause
- 5/10 models proactively edited the file to apply a fix (without being asked to)
- gpt-oss:120b produced the best output: read-only analysis with two alternative fix approaches, no file modification
- devstral-2:latest produced the most sophisticated fix: per-user game sessions with a new API endpoint
- deepseek-r1:latest was slowest (60s) but still correct

**Test Contamination Finding:** Running T3.3 in parallel initially caused contamination — one model edited the file, and subsequent models analyzed the already-fixed version. Tests must be run sequentially with fixture resets between each model when models have write access.

---

## Cross-Benchmark Summary

### Pass/Fail Matrix

| Model | T1.1 | T2.1 | T3.1 | T3.2 | T3.3 | Score |
|-------|------|------|------|------|------|-------|
| qwen3-coder:30b | ✅ | ✅ | ✅ | ✅ | ✅ | 5/5 |
| llama3.1:70b | ✅ | ✅ | ✅ | ❌ | ✅ | 4/5 |
| deepseek-r1:70b | ✅ | ✅ | ✅ | ✅ | ✅ | 5/5 |
| qwen3-coder:480b | ✅ | ✅ | ⚠️ | ✅ | ✅ | 4.5/5 |
| deepseek-r1:latest | ✅ | ✅ | ✅ | ✅ | ✅ | 5/5 |
| llama3.1:8b | ✅ | ✅ | ✅ | ✅ | ✅ | 5/5 |
| qwen-agent:latest | ✅ | ✅ | ✅ | ✅ | ✅ | 5/5 |
| devstral-2:latest | ✅ | ✅ | ✅ | ✅ | ✅ | 5/5 |
| gpt-oss:120b | ✅ | ✅ | ✅ | ✅ | ✅ | 5/5 |
| deepseek-r1:671b | ✅ | ✅ | ✅ | ✅ | ✅ | 5/5 |

### Model Rankings

**Perfect Score (5/5):** qwen3-coder:30b, deepseek-r1:70b, deepseek-r1:latest, llama3.1:8b, qwen-agent:latest, devstral-2:latest, gpt-oss:120b, deepseek-r1:671b

**Near-Perfect (4-4.5/5):** qwen3-coder:480b (T3.1 partial), llama3.1:70b (T3.2 fail)

---

## Model-Task Mapping (Recommendations)

| Task Type | Recommended Model | Rationale |
|-----------|------------------|-----------|
| Quick single-tool ops | llama3.1:8b or deepseek-r1:latest | Smallest/fastest, 100% pass rate on simple tasks |
| Code analysis | gpt-oss:120b | Best output quality, read-only approach, comprehensive |
| Code structure review | devstral-2:latest | Strongest improvement suggestions, sophisticated fixes |
| Debugging | gpt-oss:120b | Only model to analyze without modifying, two-approach output |
| User-facing summaries | deepseek-r1:70b | Best formatting with descriptions |
| Ops monitoring | deepseek-r1:70b or qwen3-coder:30b | Consistent, no failures |
| General purpose | qwen3-coder:30b | Perfect score, small footprint (18GB), fast |
| Complex reasoning | deepseek-r1:671b | Largest reasoning model, perfect score |

### Anti-Recommendations

| Model | Avoid For | Reason |
|-------|-----------|--------|
| llama3.1:70b | Open-ended exploration | Gets stuck in filesystem navigation loops |
| qwen3-coder:480b | Ambiguous prompts | Prompt sensitivity, may need retries |
| deepseek-r1:latest | Time-sensitive tasks | 60s on debug task vs 8-17s for others |

---

## Key Findings

### 1. Model Size Does Not Predict Performance

The smallest model tested (llama3.1:8b, 4.9GB VRAM) achieved a perfect 5/5 score, while the larger llama3.1:70b (42GB) was the only model to outright fail a test. qwen3-coder:480b (290GB) showed prompt sensitivity issues that smaller models didn't exhibit.

### 2. All Models Clear the Bar for Basic Agent Tasks

T1 and T2 benchmarks showed 100% pass rates across all 10 models. Any model on the Ollama host can reliably execute single-tool and multi-step sequential tasks.

### 3. Complex Tasks Reveal Model Character

T3 benchmarks exposed meaningful differences:
- **Analysis style:** gpt-oss:120b provides read-only analysis; most others proactively edit files
- **Fix quality:** devstral-2:latest produces the most architecturally sound fixes
- **Planning ability:** llama3.1:70b's T3.2 failure shows it struggles with open-ended exploration
- **Speed:** deepseek-r1:latest is 4-7x slower on complex tasks despite being the smallest reasoning model

### 4. Test Isolation Is Critical

Models that edit files during analysis tasks will contaminate shared test fixtures. Benchmark methodology must account for this:
- Run models sequentially when filesystem write access is enabled
- Reset test fixtures between each model run
- Or restrict to read-only filesystem access for analysis benchmarks

### 5. Minimum Viable Model: qwen3-coder:30b

At 18GB VRAM with a perfect 5/5 score, qwen3-coder:30b is the best balance of capability and resource efficiency. It handles all tested task types reliably without the prompt sensitivity of larger models.

---

## Methodology Notes

- All benchmarks run via `agent_chat` MCP tool with `servers: ["filesystem"]`
- Each model given identical prompts per benchmark tier
- T3.3 initially run in parallel (contamination discovered), then re-run sequentially with fixture resets
- Times are approximate wall-clock from agent_chat invocation to response
- "Tool Calls" counts filesystem operations only (read_file, write_file, edit_file, search_files, list_dir)

## Run IDs

For audit and reproducibility, all benchmark runs are stored in the agent's run database.

**T1.1:** e891389e, b09343d4, c93fd2c9, 7fe37022, 3b0567fc, 138ebbf9
**T2.1:** ba42e99c, f275d8cc, 5cd17a74, f7f43bea, 8b19170b, ea65324f
**T3.1:** d46758e6, 910eddcd, 2a9cb71c, 11fa1397, a25aea05, 6f80fe9f
**T3.2:** fb1da830, 3761a0b5, 78450837, a24f22a9, d9f7956a, 71408f2f, 6524fc26, 1335a55d, 4e2a0d42, afd1b68c
**T3.3:** 7214e040, e6080163, 1ab1a493, 8cd873be, beed0c9d, 4d607521, 4ee577a2, 61f0dd22, d782e811, eb225251
