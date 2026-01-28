# Binks Agent Benchmark Specification

## Overview
Benchmarks to evaluate LLM capability thresholds for autonomous agent tasks.

## Models Under Test
| Model | Size | Type |
|-------|------|------|
| qwen3-coder:30b | 30B | Coding specialist |
| llama3.1:70b | 70B | General purpose |
| deepseek-r1:70b | 70B | Reasoning model |
| qwen3-coder:480b | 480B MoE | Large coding specialist |
| deepseek-r1:latest | ~7B | Reasoning (small) |
| llama3.1:8b | 8B | General purpose (small) |
| qwen-agent:latest | ~32B | Agent-tuned |
| devstral-2:latest | ~120B | Coding specialist |
| gpt-oss:120b | 120B | General purpose |
| deepseek-r1:671b | 671B MoE | Large reasoning |

## Tier Definitions

### Tier 1: Simple (Single Tool)
Single tool call, no planning required.
- **T1.1**: Read a file and report its contents
- **T1.2**: List directory contents
- **T1.3**: Search for a pattern in files

### Tier 2: Multi-Step (Sequential Tools)
2-3 sequential tool calls with clear dependencies.
- **T2.1**: Read file → Edit file (version bump)
- **T2.2**: Search files → Read matching file → Summarize
- **T2.3**: List directory → Read specific file → Report

### Tier 3: Complex (Reasoning + Tools)
Multi-step with analysis and decision making.
- **T3.1**: Find all TODO comments → Categorize → Create summary
- **T3.2**: Analyze code structure → Suggest improvements
- **T3.3**: Debug task: Find error source from symptoms

## Success Criteria
- **Pass**: Task completed correctly with reasonable tool usage
- **Partial**: Task attempted but incomplete or inefficient
- **Fail**: Task not completed or wrong result

## Test Environment
- MCP Servers: filesystem, serena (for code analysis)
- Test files in: `apps/pick-a-number/` (isolated test directory)
- Ollama host: 192.168.1.4:11434
- Results: See `RESULTS_EXPANDED.md` for full benchmark data across all models
