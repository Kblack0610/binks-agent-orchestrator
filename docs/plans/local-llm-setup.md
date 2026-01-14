# Plan: Fast Local LLM Setup + Orchestrator Cleanup

*Created: 2026-01-13*

---

## Appendix: Claude Code Context Management (Reference)

How Claude Code handles long conversations (for reference when building similar features):

### 1. Automatic Summarization
When context gets too long, earlier conversation is automatically summarized. Context feels "unlimited" because it's compressed, not truncated.

### 2. On-Demand File Reading
Files loaded via `Read` tool only when needed - not dumped upfront. Keeps context lean.

### 3. MCP Tool Architecture
Tools provide *access* to resources without loading them into context. "Find references to X" returns targeted results, not whole codebase.

### 4. Manual Compaction (`/compact`)
User can trigger summarization manually to free up context mid-conversation.

### 5. Agent Delegation (`Task` tool)
Complex exploration delegated to sub-agents that return summaries, not raw data.

### Key Insight for Local Models
Local tools (aider, etc.) don't have this. They either:
- Truncate aggressively (lose context)
- Use repo-maps (shallow understanding)
- Require manual `/drop` `/add` management

To replicate in your orchestrator, you'd need a summarization step between model calls.

---

## Hardware: M3 Ultra with 512GB Unified Memory

This is an exceptional setup. You can run models that most people cannot:
- Full 405B parameter models (quantized)
- Multiple 70B+ models simultaneously
- Models with 128K+ context windows at full precision
- Essentially unlimited local AI capability

---

## Goal
1. **Phase 1**: Fastest path to local LLM with filesystem tools
2. **Phase 2**: Clean up orchestrator repo for flexibility
3. **Phase 3**: Full MCP ecosystem (future)

---

## Phase 1: Fastest Path (Do This First)

### Recommended Stack: Ollama + aider (or LM Studio for GUI)

**Why this combo:**
- Ollama: Simple, Metal-optimized, works great on Apple Silicon
- aider: Battle-tested coding assistant with filesystem + git tools built-in
- Your 512GB means you can run the BEST models with HUGE context windows

### Steps

#### 1. Get Ollama running properly
```bash
# Install/update
brew install ollama

# Start server (runs on localhost:11434)
ollama serve
```

#### 2. Pull a large-context model (YOUR HARDWARE CAN HANDLE THESE)
```bash
# RECOMMENDED: Best coding + massive context
ollama pull qwen2.5-coder:32b-instruct-q8_0    # 32K context, ~35GB, excellent quality

# ALTERNATIVE: Largest context window available
ollama pull qwen2.5:72b-instruct               # 128K context, ~45GB
ollama pull llama3.1:70b-instruct-q8_0         # 128K context, ~75GB

# OVERKILL (but you CAN run it):
ollama pull llama3.1:405b-instruct-q4_K_M      # 128K context, ~240GB
# ^ This is the largest open model. You're one of few who can run it locally.
```

#### 3. Install aider (filesystem + git tools built-in)
```bash
pip install aider-chat

# Run with your chosen model
aider --model ollama/qwen2.5-coder:32b-instruct-q8_0
```

#### 4. Verify it works
```bash
cd /home/kblack0610/dev/home/binks-agent-orchestrator
aider --model ollama/qwen2.5-coder:32b-instruct-q8_0

# Test: "read the README and summarize what this project does"
```

### Alternative: LM Studio (if you prefer GUI)
1. Download from lmstudio.ai
2. Search for and download: `Qwen2.5-Coder-32B-Instruct-GGUF` or `Llama-3.1-70B-Instruct-GGUF`
3. Load model, click "Start Server" (runs on localhost:1234)
4. Use with any OpenAI-compatible tool

### Pro Tip: Context Window Configuration
```bash
# For Ollama, you can increase context window:
# Create a Modelfile with larger context
echo 'FROM qwen2.5-coder:32b-instruct-q8_0
PARAMETER num_ctx 65536' > Modelfile.qwen-large-ctx

ollama create qwen-large-ctx -f Modelfile.qwen-large-ctx
ollama run qwen-large-ctx
```

---

## Phase 2: Orchestrator Cleanup

### Current State
- `orchestrator/cli_orchestrator/` - main orchestrator code
- 6 runners: Claude, Gemini, Ollama, Groq, OpenRouter, Factory
- MCP support is CLI pass-through only (not deep integration)

### Cleanup Tasks

1. **Fix Ollama default behavior**
   - File: `orchestrator/cli_orchestrator/runners/custom_runner.py`
   - Add sensible defaults so it works without extra config
   - Set default model to a large-context one (e.g., qwen2.5-coder:32b)

2. **Consolidate runners directory**
   - Remove or deprecate unused runners
   - Ensure `factory_runner.py` is properly integrated

3. **Create simple entry points**
   - Add `run-local.sh` script for quick local model usage
   - Add `run-mcp.sh` for MCP-enabled workflows

4. **Clean up repo structure**
   - Move experimental stuff to `experiments/`
   - Keep core orchestrator clean

### Files to Modify
- `orchestrator/cli_orchestrator/config.py` - add better defaults
- `orchestrator/cli_orchestrator/runners/custom_runner.py` - fix OllamaRunner
- `orchestrator/cli_orchestrator/main.py` - simplify CLI

---

## Phase 3: Full MCP Ecosystem (Future)

Once Phase 1 & 2 work:
- Add MCP server management to orchestrator
- Create MCP-aware Ollama runner
- Integrate with your existing MCP configs

---

## Verification

### Phase 1 Success Criteria
- [ ] `ollama serve` runs without errors
- [ ] `ollama run qwen2.5-coder:32b-instruct-q8_0` responds correctly
- [ ] `aider --model ollama/qwen2.5-coder:32b-instruct-q8_0` starts successfully
- [ ] Can read/edit files in the current directory
- [ ] Can make git commits through aider

### Phase 2 Success Criteria
- [ ] `python main.py --executor ollama-local` works with sensible defaults
- [ ] No manual model specification needed for basic usage
- [ ] Clean repo structure

---

## Model Recommendations for 512GB M3 Ultra

### Tier 1: Daily Drivers (Fast + Great Quality)
| Model | RAM Usage | Context | Speed | Best For |
|-------|-----------|---------|-------|----------|
| qwen2.5-coder:32b-instruct-q8_0 | ~35GB | 32K (expandable to 128K) | Fast | Coding |
| deepseek-coder-v2:236b | ~140GB | 128K | Medium | Complex coding |
| llama3.1:70b-instruct-q8_0 | ~75GB | 128K | Medium | General + coding |

### Tier 2: Maximum Power (You can run these!)
| Model | RAM Usage | Context | Speed | Best For |
|-------|-----------|---------|-------|----------|
| llama3.1:405b-instruct-q4_K_M | ~240GB | 128K | Slow | Best open model |
| qwen2.5:72b-instruct | ~45GB | 128K | Medium | General intelligence |
| mixtral-8x22b-instruct | ~90GB | 64K | Medium | Diverse tasks |

### Tier 3: Run Multiple Simultaneously
With 512GB you can run MULTIPLE models at once:
- Keep a fast model (32B) for quick tasks
- Keep a large model (70B+) for complex reasoning
- Your orchestrator can route to the right one!

---

## Context Window Reality Check

With 512GB RAM, context windows are NOT your bottleneck:

| Model | Default Context | Max Expandable | Your RAM Can Handle |
|-------|-----------------|----------------|---------------------|
| qwen2.5-coder:32b | 32K | 128K | Yes, easily |
| llama3.1:70b | 128K | 128K (native) | Yes |
| llama3.1:405b | 128K | 128K (native) | Yes |

**128K tokens ≈ 100,000 words ≈ a full novel ≈ entire medium codebase**

You should NOT have context problems with proper model selection.

---

## Analysis: Why You've Had Context Problems (And How to Fix Them)

### The Real Issue: Default Configurations Are Terrible

Most tools ship with conservative defaults for low-RAM machines. With 512GB, these defaults actively hurt you:

| Tool | Default Behavior | What It Should Be For You |
|------|------------------|---------------------------|
| Ollama | `num_ctx: 2048` (tiny!) | `num_ctx: 65536` or higher |
| aider | Truncates aggressively | Can handle full context |
| LM Studio | Conservative memory limits | Max it out |

### Problem 1: Ollama's Default Context is 2048 Tokens

**This is why you've had problems.** Even if you pull a 70B model, Ollama defaults to a 2048 token context window unless you configure it.

**Fix:**
```bash
# Option A: Set per-session
ollama run llama3.1:70b --num-ctx 65536

# Option B: Create a custom model with large context (permanent)
cat << 'EOF' > ~/Modelfile.bigctx
FROM llama3.1:70b-instruct-q8_0
PARAMETER num_ctx 131072
PARAMETER num_gpu 999
EOF
ollama create llama3.1-bigctx -f ~/Modelfile.bigctx

# Option C: Set in Ollama config (affects all models)
# Add to ~/.ollama/config or set OLLAMA_NUM_CTX=65536
```

### Problem 2: aider's Context Management

aider truncates conversation history aggressively by default. With your RAM:

```bash
# Tell aider you have plenty of context
aider --model ollama/llama3.1-bigctx \
      --map-tokens 4096 \
      --max-chat-history-tokens 65536
```

### Problem 3: Model Selection

Many "coding" models have small context:
- `codellama:34b` - only 16K context
- `deepseek-coder:33b` - only 16K context
- `starcoder2:15b` - only 16K context

**Use these instead:**
- `qwen2.5-coder:32b` - 32K expandable to 128K
- `llama3.1:70b` - native 128K
- `deepseek-coder-v2:236b` - 128K (you can run this!)

---

## Tool Comparison: Why Not Kilo Code / Continue / Others?

| Tool | Pros | Cons | Good For You? |
|------|------|------|---------------|
| **aider** | Mature, git-aware, filesystem tools, Ollama support | CLI only, learning curve | Yes - best for coding |
| **Continue.dev** | VS Code native, nice UI | Context = open files only, less mature | Maybe - for quick edits |
| **Kilo Code** | VS Code, local model support | Early stage, less features | Not yet - too immature |
| **LM Studio** | GUI, easy model management | No coding tools built in | Yes - as backend only |
| **Open WebUI** | Web UI, tools support | Requires separate setup | Yes - if you want UI |
| **Claude Code** | Best MCP support, auto-summarization | Cloud-dependent (API costs) | Yes - for complex work |

### My Recommendation For Your Setup

```
┌─────────────────────────────────────────────────────────────┐
│                    Your AI Stack                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │   Ollama    │    │   aider     │    │ Claude Code │     │
│  │  (backend)  │───▶│ (coding)    │    │ (MCP work)  │     │
│  │ 70B+ models │    │ filesystem  │    │ full tools  │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│         │                                     │             │
│         ▼                                     ▼             │
│  ┌─────────────┐                    ┌─────────────────┐    │
│  │ LM Studio   │                    │ Your Orchestrator│    │
│  │ (GUI/quick) │                    │ (multi-model)    │    │
│  └─────────────┘                    └─────────────────┘    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

- **Daily coding**: aider + Ollama (qwen2.5-coder:32b or llama3.1:70b)
- **Complex MCP workflows**: Claude Code (what you're in now)
- **GUI exploration**: LM Studio
- **Multi-model orchestration**: Your binks-agent-orchestrator (after cleanup)

---

## Quick Start Commands (Copy-Paste Ready)

### Setup (run once)
```bash
# 1. Install/update Ollama
brew install ollama

# 2. Start Ollama server
ollama serve &

# 3. Pull recommended model
ollama pull qwen2.5-coder:32b-instruct-q8_0

# 4. Create large-context version
cat << 'EOF' > ~/Modelfile.qwen-bigctx
FROM qwen2.5-coder:32b-instruct-q8_0
PARAMETER num_ctx 65536
PARAMETER num_gpu 999
EOF
ollama create qwen-bigctx -f ~/Modelfile.qwen-bigctx

# 5. Install aider
pip install aider-chat
```

### Daily Use
```bash
# Quick coding session
aider --model ollama/qwen-bigctx

# With extra context headroom
aider --model ollama/qwen-bigctx --map-tokens 4096 --max-chat-history-tokens 65536

# For a specific project
cd ~/your-project && aider --model ollama/qwen-bigctx
```
