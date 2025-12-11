# CLI Orchestrator Architecture

## Core Design Principle: Agnostic Composability

Roles and Runners are ORTHOGONAL concerns.

```
┌─────────────────────────────────────────────────────────────┐
│  RUNNERS (How to talk to CLIs)                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │ Claude   │ │ Gemini   │ │ Ollama   │ │ KiloCode │  ...  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
└─────────────────────────────────────────────────────────────┘
                            ↓ (any runner)
┌─────────────────────────────────────────────────────────────┐
│  AGENT (Composable: Role + Runner + System Prompt)          │
│                                                             │
│  Agent(name, role, runner, system_prompt)                   │
│                                                             │
│  Examples:                                                  │
│  - Agent("arch", ARCHITECT, gemini_runner, architect_prompt)│
│  - Agent("arch", ARCHITECT, claude_runner, architect_prompt)│
│  - Agent("impl", EXECUTOR, claude_runner, executor_prompt)  │
│  - Agent("impl", EXECUTOR, ollama_runner, executor_prompt)  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  ORCHESTRATOR (Workflows using configured agents)           │
│                                                             │
│  moa_workflow(architect=agent1, executor=agent2, ...)       │
└─────────────────────────────────────────────────────────────┘
```

## Hybrid Persistence Architecture

Three layers work together:

| Layer | Storage | Purpose | Managed By |
|-------|---------|---------|------------|
| **Agent State** | `SqliteAgentStorage` (Agno Native) | Manager's brain, crash recovery, conversation history | Agno framework |
| **CLI Sessions** | `.cli_sessions.json` | Bridge to CLI tools, session IDs | HeadlessCliTools |
| **Project Context** | `.orchestrator/*.md` | Source of truth, Git-tracked, human-readable | MemoryBank |

### Why This Split?

- **Agno Native**: Don't reinvent crash recovery - Agno handles it
- **CLI Sessions**: Agno doesn't know about Claude CLI's internal session IDs
- **Memory Bank**: Vector DBs are bad for active coding state - files live in Git

## Memory Bank Files

Located in `.orchestrator/`:

```
.orchestrator/
├── productContext.md    # Goal, project info, success criteria (stable)
├── activeContext.md     # Current state, progress, blockers (changes often)
├── systemPatterns.md    # Architectural decisions, patterns (grows over time)
└── progress/
    └── {timestamp}.md   # Historical snapshots (optional)
```

## Agent Roles

Semantic organization (not behavior):

| Role | Typical Use | Example Prompt Focus |
|------|-------------|---------------------|
| ARCHITECT | Planning, design, review | "Break down tasks, define interfaces" |
| EXECUTOR | Implementation | "Write clean, working code" |
| CRITIC | Code review | "Evaluate quality, give PASS/FAIL" |
| RESEARCHER | Information gathering | "Analyze options, recommend" |
| TESTER | Verification | "Run tests, validate behavior" |

## Usage Examples

### Basic Agent Creation

```python
from runners import ClaudeRunner, GeminiRunner
from agent import Agent, AgentRole, create_agent

# Create with factory (uses default prompts)
architect = create_agent("arch", GeminiRunner(), AgentRole.ARCHITECT)
executor = create_agent("impl", ClaudeRunner(), AgentRole.EXECUTOR)

# Swap backends easily
architect_v2 = architect.with_runner(ClaudeRunner())
```

### MoA Workflow

```python
from orchestrator import Orchestrator

orch = Orchestrator()
result = orch.run_moa_workflow(
    goal="Build REST API",
    architect=architect,
    executor=executor,
    critic=critic  # Optional, defaults to architect
)
```

### CLI Usage

```bash
# Start new MoA workflow
python main.py --moa "Build a REST API for user management"

# Resume interrupted task
python main.py --resume

# Check state
python main.py --status
```

## Agno Migration Path

When ready to migrate to full Agno:

| Current (CLI Orchestrator) | Future (Agno) |
|---------------------------|---------------|
| `Agent` | `agno.Agent` |
| `CLIRunner` | `agno.Model` or `agno.Tool` |
| `system_prompt` | `instructions` |
| `invoke()` | `run()` |

```python
# Current
from agent import Agent
from runners import ClaudeRunner

agent = Agent(name="architect", runner=ClaudeRunner(), ...)
response = agent.invoke("Design a REST API")

# Future (Agno)
from agno.agent import Agent
from agno.models.anthropic import Claude

agent = Agent(name="architect", model=Claude(...), ...)
response = agent.run("Design a REST API")
```

## File Overview

| File | Lines | Purpose |
|------|-------|---------|
| `agent.py` | ~450 | Agnostic Agent class + factory |
| `memory_bank.py` | ~330 | File-based context management |
| `tools/headless_tools.py` | ~400 | CLI wrappers with session persistence |
| `runners/base.py` | ~100 | CLIRunner base class |
| `runners/claude_runner.py` | ~200 | Claude CLI wrapper |
| `orchestrator.py` | ~400 | Workflow orchestration |
