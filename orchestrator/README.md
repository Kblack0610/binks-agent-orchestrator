# Orchestrator

Multi-agent workflow orchestration layer for binks-agent. Execute dev workflows like planning, implementation, and review with human-in-loop checkpoints.

## Overview

The orchestrator coordinates multiple specialized agents in sequential workflows:

```
┌─────────┐     ┌────────────┐     ┌─────────────┐     ┌──────────┐
│ Planner │────▶│ Checkpoint │────▶│ Implementer │────▶│ Reviewer │
└─────────┘     └────────────┘     └─────────────┘     └──────────┘
   qwen3           human              qwen3              qwen3
  (analyze)       (approve)          (code)            (verify)
```

Each agent can use different LLM models and has role-specific system prompts and tool access.

## Quick Start

```bash
# Build
cargo build --release -p orchestrator

# List available workflows
./target/release/orchestrator workflow list

# Run a workflow
./target/release/orchestrator workflow run implement-feature --task "Add dark mode toggle"

# Run without human approval prompts
./target/release/orchestrator workflow run quick-fix --task "Fix typo" --non-interactive
```

## Built-in Workflows

| Workflow | Description | Steps |
|----------|-------------|-------|
| `implement-feature` | Plan, implement, and review a new feature | planner → checkpoint → implementer → reviewer |
| `fix-bug` | Investigate, fix, and test a bug | investigator → checkpoint → implementer → tester |
| `refactor` | Plan and execute a refactoring | planner → checkpoint → implementer → reviewer |
| `quick-fix` | Quick fix without planning | implementer → tester |

## Agents

| Agent | Role | Default Model | Tools |
|-------|------|---------------|-------|
| `planner` | Analyzes tasks, creates implementation plans | qwen3-coder:30b | filesystem, serena |
| `implementer` | Makes code changes based on plans | qwen3-coder:30b | filesystem, serena, github-gh |
| `reviewer` | Reviews changes, suggests improvements | qwen3-coder:30b | filesystem, serena, github-gh |
| `investigator` | Investigates bugs, finds root causes | qwen3-coder:30b | filesystem, serena, sysinfo |
| `tester` | Runs tests, verifies changes | qwen3-coder:30b | filesystem, serena |

View agent details:
```bash
./target/release/orchestrator agents list
./target/release/orchestrator agents show planner
```

## Custom Workflows

Create custom workflows as TOML files in:
- `~/.config/binks/workflows/` (global)
- `.binks/workflows/` (project-local)

### Example: Custom Review Workflow

```toml
# ~/.config/binks/workflows/security-review.toml
name = "security-review"
description = "Security-focused code review"

[[steps]]
type = "agent"
name = "investigator"
task = "Analyze for security vulnerabilities: {task}"

[[steps]]
type = "checkpoint"
message = "Review security findings before proceeding?"
show = "investigation"

[[steps]]
type = "agent"
name = "implementer"
task = "Fix the identified security issues"
```

### Workflow Step Types

**Agent Step** - Run an agent with a task:
```toml
[[steps]]
type = "agent"
name = "planner"           # Agent name from registry
task = "Analyze: {task}"   # Task with variable substitution
model = "llama3.1:70b"     # Optional model override
```

**Checkpoint Step** - Human approval gate:
```toml
[[steps]]
type = "checkpoint"
message = "Approve the plan?"
show = "plan"              # Optional: show context variable
```

### Variable Substitution

Context variables are passed between steps:

| Variable | Source | Description |
|----------|--------|-------------|
| `{task}` | CLI input | Original task from `--task` |
| `{plan}` | planner agent | Implementation plan |
| `{investigation}` | investigator agent | Bug analysis |
| `{changes}` | implementer agent | Code changes made |
| `{review}` | reviewer agent | Review feedback |
| `{test_results}` | tester agent | Test output |

## CLI Reference

```bash
# Workflow commands
orchestrator workflow list                    # List all workflows
orchestrator workflow show <name>             # Show workflow steps
orchestrator workflow run <name> --task "..." # Execute workflow

# Workflow options
--task, -t           # Task description (required for run)
--workflows-dir      # Custom workflows directory
--non-interactive    # Auto-approve all checkpoints
--verbose, -v        # Enable verbose output

# Agent commands
orchestrator agents list                      # List all agents
orchestrator agents show <name>               # Show agent config

# Global options
--ollama-url         # Override Ollama URL
--model              # Override default model
```

## Configuration

The orchestrator uses settings from `agent/.agent.toml`:

```toml
[llm]
url = "http://192.168.1.4:11434"
model = "qwen3-coder:30b"
```

Override via CLI or environment:
```bash
export OLLAMA_URL=http://localhost:11434
export OLLAMA_MODEL=llama3.1:8b
```

## Architecture

```
orchestrator/
├── src/
│   ├── lib.rs           # Public API
│   ├── agent_config.rs  # AgentConfig, AgentRegistry
│   ├── workflow.rs      # WorkflowStep, Workflow, TOML parsing
│   ├── engine.rs        # WorkflowEngine execution
│   ├── checkpoint.rs    # Human-in-loop approval
│   ├── main.rs          # CLI binary
│   └── prompts/         # Agent system prompts
│       ├── planner.rs
│       ├── implementer.rs
│       ├── reviewer.rs
│       ├── investigator.rs
│       └── tester.rs
```

## Library Usage

```rust
use orchestrator::{WorkflowEngine, AgentRegistry};
use orchestrator::engine::EngineConfig;

// Create engine with defaults
let engine = WorkflowEngine::with_defaults()?;

// Or customize
let registry = AgentRegistry::with_defaults("qwen3-coder:30b");
let config = EngineConfig {
    ollama_url: "http://localhost:11434".to_string(),
    default_model: "llama3.1:8b".to_string(),
    non_interactive: false,
    verbose: true,
    ..Default::default()
};
let engine = WorkflowEngine::new(registry, config);

// Run a workflow
let result = engine.run("implement-feature", "Add dark mode").await?;

println!("Status: {:?}", result.status);
for step in result.step_results {
    println!("Step {}: {}ms", step.step_index, step.duration_ms);
}
```

## Roadmap

- [x] Sequential workflow execution
- [x] Human-in-loop checkpoints
- [x] Per-agent model configuration
- [x] Custom TOML workflows
- [ ] Parallel step execution
- [ ] Conditional branching
- [ ] Workflow persistence/resume
