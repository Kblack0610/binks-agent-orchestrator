# System Patterns

## Architectural Decisions

### Role-Based Architecture

All agent interactions are role-based for consistency with Agno logic and the scoring system.

**Core Principle**: Roles and Runners are ORTHOGONAL concerns.
- A role defines WHAT an agent does (semantic behavior via system prompt)
- A runner defines HOW to communicate (Claude CLI, Gemini CLI, Ollama, etc.)
- Any role can use any runner

```
User Request
     ↓
┌─────────────┐
│   TRIAGE    │ ← Workflow router: selects appropriate workflow
└─────────────┘
     ↓
  Selects workflow based on task analysis
     ↓
┌────────────────────────────────────────────────────────────────────┐
│  QUICK    │ SIMPLE   │ STANDARD           │ DEBUG              │ ... │
│  []       │ executor │ architect →        │ debugger →         │     │
│  (direct) │          │ executor →         │ executor →         │     │
│           │          │ critic             │ verifier           │     │
└────────────────────────────────────────────────────────────────────┘
```

### Predefined Workflows

| Workflow | Roles | Use Case |
|----------|-------|----------|
| **QUICK** | `[]` | Simple questions, math, definitions |
| **SIMPLE** | `executor` | Small, well-defined code changes |
| **STANDARD** | `architect → executor → critic` | Features needing design |
| **FULL** | `planner → architect → executor → critic → gatekeeper → judge` | Complex systems |
| **DEBUG** | `debugger → executor → verifier` | Bug investigation |
| **RESEARCH** | `researcher → documenter` | Information gathering |
| **REVIEW** | `critic → gatekeeper → judge` | Code reviews/audits |
| **TEST** | `tester → executor → verifier` | Test creation |

### Separation of Concerns

| System | Purpose | Roles Involved |
|--------|---------|----------------|
| **Workflow** | Task execution | Planner, Architect, Executor, Critic |
| **Routing** | Workflow selection | Triage |
| **Evaluation** | Quality scoring | Gatekeeper, Judge |
| **Utilities** | Specialized tasks | Debugger, Tester, Verifier, Researcher, Documenter |
| **Benchmarking** | Model selection | All roles (for per-role benchmarks) |

### Triage Logic

The Triage role routes requests by selecting a workflow:

```
Task arrives → Triage analyzes → Outputs JSON:
{
  "workflow": "STANDARD",
  "roles": ["architect", "executor", "critic"],
  "reasoning": "Feature needs design but isn't complex enough for full workflow",
  "answer": null
}
```

**Workflow Selection Criteria**:
- **QUICK**: Questions, math, definitions → answer directly
- **SIMPLE**: Small, clear changes → just execute
- **STANDARD**: Features needing design → architect first
- **FULL**: Complex/unclear requirements → full planning and evaluation
- **DEBUG**: Bugs, errors, stack traces → diagnose first
- **RESEARCH**: Information gathering → researcher first
- **REVIEW**: Code reviews, audits → critic first
- **TEST**: Test creation → tester first
- **CUSTOM**: None of the above → specify custom role sequence

---

## Code Patterns

### Agent Creation Pattern

```python
from agent import create_agent, AgentRole
from runners import ClaudeRunner, GeminiRunner

# Factory with default prompts per role
architect = create_agent("arch", GeminiRunner(), AgentRole.ARCHITECT)
executor = create_agent("impl", ClaudeRunner(), AgentRole.EXECUTOR)
critic = create_agent("critic", ClaudeRunner(), AgentRole.CRITIC)

# Swap backends easily
architect_v2 = architect.with_runner(ClaudeRunner())
```

### Evaluation Pattern (Meritocratic System)

```python
from model_evaluator import Gatekeeper, Judge, ModelSelector

# Two-stage evaluation
gatekeeper = Gatekeeper()
gate_result = gatekeeper.validate(response, requirements)

if gate_result.passed:
    judge = Judge(runner)
    judge_result = judge.evaluate(response, prompt, rubric)

    # Record for future model selection
    store.record(EvaluationResult(
        role=role,
        model=model,
        gatekeeper=gate_result,
        judge=judge_result
    ))

# Select best model per role
selector = ModelSelector(store)
best_backend, best_model = selector.select("architect")
```

### Workflow Routing Pattern

```python
from cli_orchestrator import WORKFLOWS, ROLE_DESCRIPTIONS, create_triage
from cli_orchestrator.runners import ClaudeRunner
import json

# Create triage agent
runner = ClaudeRunner()
triage = create_triage(runner)

# Route a task
task = "Build authentication system with OAuth and JWT"
response = triage.invoke(task)

# Parse triage decision
decision = json.loads(response.content)
# decision = {
#     "workflow": "FULL",
#     "roles": ["planner", "architect", "executor", "critic", "gatekeeper", "judge"],
#     "reasoning": "Complex system with multiple components, needs full planning",
#     "answer": None
# }

# Get workflow details
workflow = WORKFLOWS[decision["workflow"].lower()]
print(f"Workflow: {workflow['description']}")
print(f"Roles: {workflow['roles']}")

# Execute the workflow roles in sequence
for role_name in decision["roles"]:
    print(f"Executing: {role_name} - {ROLE_DESCRIPTIONS[role_name]}")
```

### MoA Workflow Pattern

```python
from orchestrator import Orchestrator

orch = Orchestrator()
result = orch.run_moa_workflow(
    goal="Build feature X",
    architect=architect,
    executor=executor,
    critic=critic,  # Optional, defaults to architect
    max_iterations=5
)
```

---

## API Contracts

### Agent Interface

```python
class Agent:
    def invoke(self, prompt: str, context: str = None) -> AgentResponse
    def with_runner(self, runner: CLIRunner) -> Agent
    def with_prompt(self, prompt: str) -> Agent
```

### Evaluation Interface

```python
class Gatekeeper:
    def validate(self, response: str, requirements: Dict) -> GatekeeperResult

class Judge:
    def evaluate(self, response: str, prompt: str, rubric: Dict) -> JudgeResult

class ModelSelector:
    def select(self, role: str) -> Tuple[str, str]  # (backend, model)
```

### Role Benchmark Contract

Each role has a benchmark definition:

```python
ROLE_BENCHMARKS = {
    "role_name": {
        "prompt": str,              # Test prompt
        "requirements": Dict,        # Gatekeeper checks
        "rubric": Dict[str, str],   # Judge criteria
    }
}
```

---

## Lessons Learned

### 2024-12-12: Missing Integrations

**Issue**: Gatekeeper/Judge evaluation system exists but is NOT integrated into orchestrator workflow.

**Root Cause**: Components were built separately without integration layer.

**Resolution**: Need to:
1. Add Triage role to entry point
2. Wire Gatekeeper/Judge into MoA workflow post-review
3. Record evaluation evidence to ScoreStore

### 2024-12-12: Critic Role Defaulting

**Issue**: Critic role defaults to Architect when not explicitly provided.

**Location**: `orchestrator.py:387` - `critic = critic or architect`

**Decision**: Keep this as a fallback but document that separate Critic is recommended for better evaluation quality.

### 2024-12-12: Documentation Gap

**Issue**: `systemPatterns.md` was empty placeholder, architectural decisions not recorded.

**Resolution**: This document now captures the intended architecture and patterns.
