# CLI Orchestrator

Multi-agent headless orchestration using Claude CLI, Gemini, and custom modules.

## Quick Start

```bash
cd orchestrator/cli_orchestrator

# Check available backends
python main.py --check

# Run interactive mode
python main.py

# Run a predefined workflow
python main.py --workflow design-implement-review "Build a REST API for user management"

# Benchmark multiple backends
python main.py --benchmark "Explain microservices architecture"
```

## Architecture

```
cli_orchestrator/
├── runners/              # Backend abstractions
│   ├── base.py          # CLIRunner abstract base
│   ├── claude_runner.py # Claude CLI + SuperClaude
│   ├── gemini_runner.py # Gemini API/CLI
│   └── custom_runner.py # Custom Python/scripts
├── profiles.py          # Agent profiles (SuperClaude mapping)
├── orchestrator.py      # Multi-agent conversation loop
├── benchmark.py         # Backend comparison tools
└── main.py             # CLI entry point
```

## Usage Examples

### 1. Basic Multi-Agent Workflow

```python
from runners import ClaudeRunner
from profiles import create_default_registry
from orchestrator import Orchestrator, WorkflowBuilder

# Setup
claude = ClaudeRunner()
registry = create_default_registry(claude)
orchestrator = Orchestrator(registry, debug=True)

# Run workflow: Design -> Implement -> Review
conversation = (WorkflowBuilder(orchestrator)
    .goal("Build a caching layer")
    .then("architect", "Design the caching architecture")
    .then("implementer", "Implement the design")
    .then("reviewer", "Review for best practices")
    .build())

# Access results
for turn in conversation.turns:
    print(f"[{turn.agent_name}] {turn.response[:500]}...")
```

### 2. Using SuperClaude Profiles

```python
from runners import ClaudeRunner

claude = ClaudeRunner()

# Direct SuperClaude commands
result = claude.design("a microservices architecture for e-commerce")
result = claude.implement("the authentication service")
result = claude.troubleshoot("why the database connection is slow")
result = claude.research("best practices for API rate limiting")
```

### 3. Multi-Backend Benchmarking

```python
from runners import ClaudeRunner, GeminiRunner
from benchmark import Benchmarker, quality_indicators_scorer

benchmarker = Benchmarker(debug=True)
benchmarker.add_runner(ClaudeRunner(output_format="text"))
benchmarker.add_runner(GeminiRunner(backend="api"))

comparison = benchmarker.run(
    "What are the key principles of clean architecture?",
    scorer=quality_indicators_scorer
)

print(comparison.summary())
```

### 4. Custom Agent Integration

```python
from runners import CustomRunner
from profiles import AgentProfile, AgentRole

# Create from Python function
def my_linter(code: str) -> str:
    # Your custom analysis
    return f"Analysis of: {code[:100]}..."

linter = CustomRunner.from_callable(my_linter, name="linter")

# Create from shell script
formatter = CustomRunner.from_executable("./scripts/format.sh", name="formatter")

# Add to registry
profile = AgentProfile(
    name="linter",
    role=AgentRole.REVIEWER,
    runner=linter,
    description="Static code analyzer"
)
registry.register(profile)
```

### 5. Iterative Agent Loop

```python
# Agents iterate until a stop condition
conversation = orchestrator.run_loop(
    goal="Design and refine a caching strategy",
    agents=["architect", "reviewer"],
    initial_prompt="Design a caching strategy for high-traffic API",
    stop_condition=lambda c: "approved" in c.get_last_response().lower(),
    max_iterations=5
)
```

## Available Profiles

These map to SuperClaude commands:

| Profile | Command | Role |
|---------|---------|------|
| architect | /sc:design | System design |
| implementer | /sc:implement | Code implementation |
| analyzer | /sc:analyze | Code analysis |
| tester | /sc:test | Testing/QA |
| researcher | /sc:research | Information gathering |
| troubleshooter | /sc:troubleshoot | Problem diagnosis |
| documenter | /sc:document | Documentation |
| reviewer | /sc:analyze | Code review |

## Predefined Workflows

```python
from orchestrator import (
    design_implement_review,   # Standard feature dev
    research_design_implement, # Exploring new domains
    debug_fix_test            # Bug fixing
)

# Run a workflow
conversation = design_implement_review(orchestrator, "Build OAuth2 integration")
```

## Backend Configuration

### Claude CLI (Required)
```bash
npm install -g @anthropic-ai/claude-code
```

### Gemini API (Optional)
```bash
pip install google-generativeai
export GOOGLE_API_KEY="your-api-key"
```

### Ollama (Optional)
```bash
# Install Ollama
curl https://ollama.ai/install.sh | sh
ollama pull llama3.1:8b
```

## Future Integration with Agno

When you're ready to migrate to Agno, the profiles translate directly:

```python
# Current (CLI Orchestrator)
claude = ClaudeRunner()
result = claude.design("Build a REST API")

# Future (Agno)
from agno.agent import Agent
from agno.models.anthropic import Claude

architect = Agent(
    name="Architect",
    model=Claude(id="claude-3-5-sonnet-20241022"),
    instructions=[...SuperClaude design prompt...]
)
response = architect.run("Build a REST API")
```

The workflow patterns and profiles you develop here will transfer directly to Agno agents.
