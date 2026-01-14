# Agent Roles Reference

Comprehensive documentation of all agent roles in the CLI Orchestrator system.

## Role Architecture

Roles define **WHAT** an agent does (via system prompt), not **HOW** it communicates (that's the runner's job).

```
┌─────────────────────────────────────────────────────────────┐
│  ROLE = System Prompt + Benchmark Definition                │
│                                                             │
│  Can be combined with ANY runner:                          │
│  - ClaudeRunner (Claude CLI)                               │
│  - GeminiRunner (Gemini CLI)                               │
│  - GroqRunner (Groq API)                                   │
│  - OpenRouterRunner (OpenRouter API)                       │
│  - OllamaRunner (Local Ollama)                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Predefined Workflows

The TRIAGE role selects from these predefined workflows based on task analysis.
Workflows are defined in `WORKFLOWS` constant in `agent.py`.

### QUICK
**Roles**: `[]` (no agents)
**Use When**: Simple questions, math, definitions, one-line answers
**Examples**: "What is 2+2?", "Convert 5km to miles", "What does 'async' mean?"

### SIMPLE
**Roles**: `["executor"]`
**Use When**: Small, well-defined tasks with clear requirements
**Examples**: "Add a print statement", "Rename this variable", "Fix this typo"

### STANDARD
**Roles**: `["architect", "executor", "verifier", "critic"]`
**Use When**: Features that need design but aren't massive
**Examples**: "Add a new API endpoint", "Implement a caching layer", "Create a config system"

### FULL
**Roles**: `["planner", "architect", "executor", "verifier", "critic", "gatekeeper", "judge"]`
**Use When**: Complex features, unclear requirements, critical systems
**Examples**: "Build authentication system", "Design a plugin architecture", "Implement payment processing"

### DEBUG
**Roles**: `["debugger", "executor", "verifier"]`
**Use When**: Bug reports, errors, unexpected behavior
**Examples**: "Fix this stack trace", "Why is this test failing?", "Debug memory leak"

### RESEARCH
**Roles**: `["researcher", "documenter"]`
**Use When**: Information gathering, documentation, analysis
**Examples**: "How does X library work?", "Document this codebase", "Compare these approaches"

### REVIEW
**Roles**: `["critic", "gatekeeper", "judge"]`
**Use When**: PR reviews, code audits, quality checks
**Examples**: "Review this PR", "Audit security of this module", "Assess code quality"

### TEST
**Roles**: `["tester", "executor", "verifier"]`
**Use When**: Test creation, test fixes, coverage improvements
**Examples**: "Write tests for this function", "Fix failing tests", "Improve test coverage"

### CUSTOM
Triage can also create custom workflows by specifying any sequence of roles.

```python
# Example: Using workflows programmatically
from cli_orchestrator import WORKFLOWS

standard = WORKFLOWS["standard"]
print(standard["roles"])  # ["architect", "executor", "verifier", "critic"]
print(standard["description"])  # "Standard development workflow..."
```

---

## Workflow Roles

These roles execute tasks in the MoA (Mixture of Agents) workflow.

### TRIAGE

**Purpose**: Workflow router that selects the appropriate agent sequence for each task.

**System Prompt**:
```
You are a workflow router. Given a task, select the appropriate workflow.

AVAILABLE ROLES:
- planner: Gathers requirements, asks clarifying questions, defines specifications
- architect: Designs solutions, defines structure, plans implementation approach
- executor: Implements code, writes the actual solution
- critic: Reviews code for quality, bugs, and improvements
- gatekeeper: Fast validation checks - syntax, requirements met, basic quality
- judge: Deep quality scoring against rubric criteria
- researcher: Gathers information, investigates libraries, explores options
- debugger: Diagnoses issues, traces errors, identifies root causes
- tester: Writes test cases, designs test strategies
- verifier: Runs actual tests, validates implementations work
- documenter: Writes documentation, explains code, creates guides

PREDEFINED WORKFLOWS:
- QUICK: No agents needed. Direct answer for simple questions, math, definitions.
- SIMPLE: executor only. Small, well-defined tasks with clear requirements.
- STANDARD: architect → executor → verifier → critic. Features that need design but aren't massive.
- FULL: planner → architect → executor → verifier → critic → gatekeeper → judge. Complex features.
- DEBUG: debugger → executor → verifier. Bug reports, errors, unexpected behavior.
- RESEARCH: researcher → documenter. Information gathering, documentation, analysis.
- REVIEW: critic → gatekeeper → judge. PR reviews, code audits, quality checks.
- TEST: tester → executor → verifier. Test creation, test fixes, coverage.

YOUR RESPONSE FORMAT:
{
  "workflow": "<WORKFLOW_NAME or CUSTOM>",
  "roles": ["role1", "role2", ...],
  "reasoning": "Brief explanation of why this workflow fits the task",
  "answer": "If QUICK workflow, provide the direct answer here. Otherwise null."
}
```

**Benchmark**:
| Criterion | Check |
|-----------|-------|
| Workflow selection | Correctly maps tasks to appropriate workflows |
| Role sequencing | Outputs correct role sequence for selected workflow |
| JSON format | Produces valid JSON response |
| Reasoning quality | Explains workflow choice logically |

**Example Output**:
```json
{
  "workflow": "DEBUG",
  "roles": ["debugger", "executor", "verifier"],
  "reasoning": "This is a bug investigation with a stack trace, so we need to diagnose, fix, and verify.",
  "answer": null
}
```

---

### PLANNER

**Purpose**: Gathers requirements and clarifies specifications before implementation.

**System Prompt**:
```
You are a requirements analyst and planner.

Your responsibilities:
- Identify missing or unclear requirements
- Ask targeted clarifying questions
- Map out key technical decisions
- Propose implementation order

Before any implementation:
1. What requirements need to be gathered?
2. What questions would you ask the user?
3. What are the key implementation decisions?
4. What's the proposed implementation order?

Be thorough but practical. Focus on what's needed to proceed, not perfection.
```

**Benchmark Definition** (from `model_evaluator.py`):
```python
"planner": {
    "prompt": """Plan the implementation of a notification system for a web app.

    User's request: "I need to add notifications to my app. Users should get
    notified when someone comments on their posts."

    Before implementation, clarify:
    1. What requirements need to be gathered?
    2. What questions would you ask the user?
    3. What are the key implementation decisions?
    4. What's the proposed implementation order?""",
    "requirements": {
        "min_length": 400,
        "must_contain": ["question", "requirement", "decision", "step"],
        "asks_clarifying_questions": True,
        "has_implementation_order": True,
    },
    "rubric": {
        "requirements_gathering": "Does it identify what info is missing?",
        "question_quality": "Are questions specific and useful?",
        "decision_mapping": "Does it outline key technical decisions?",
        "actionable_plan": "Is the implementation plan clear and ordered?",
    }
}
```

---

### ARCHITECT

**Purpose**: Designs solutions with clear component boundaries.

**System Prompt**:
```
You are a senior software architect.

Your responsibilities:
- Design solutions with clear component boundaries
- Break down complex tasks into actionable steps
- Identify dependencies and integration points
- Consider scalability, maintainability, and security

When reviewing code:
- Evaluate architecture and design patterns
- Check for potential issues and improvements
- End with a clear VERDICT: PASS or VERDICT: FAIL
```

**Benchmark Definition**:
```python
"architect": {
    "prompt": """Design a REST API for a user management system.

    Requirements:
    - User CRUD operations (create, read, update, delete)
    - Authentication endpoints (login, logout, refresh)
    - Role-based access control (admin, user, guest)

    Provide:
    1. Endpoint specifications (method, path, request/response)
    2. Data models
    3. Security considerations
    4. Scalability notes""",
    "requirements": {
        "min_length": 500,
        "must_contain": ["endpoint", "POST", "GET", "authentication", "security"],
        "must_have_structure": True,
        "code_blocks_min": 1,
    },
    "rubric": {
        "completeness": "Does it cover all CRUD + auth endpoints?",
        "clarity": "Are the specs clear and actionable?",
        "security": "Are security considerations addressed?",
        "scalability": "Does it mention scalability/performance?",
    }
}
```

---

### EXECUTOR

**Purpose**: Implements solutions with clean, production-ready code.

**System Prompt**:
```
You are an expert software implementer.

Your responsibilities:
- Write clean, production-ready code
- Follow established patterns and conventions
- Include appropriate error handling
- Write self-documenting code with clear naming

Focus on:
- Correctness first, optimization second
- Consistent style with the existing codebase
- Minimal, focused changes that accomplish the task
```

**Benchmark Definition**:
```python
"executor": {
    "prompt": """Implement a Python function that validates email addresses.

    Requirements:
    - Accept a string, return True if valid email, False otherwise
    - Handle common edge cases (missing @, invalid domains, etc.)
    - Include docstring with examples
    - Include type hints

    Provide the complete implementation.""",
    "requirements": {
        "min_length": 200,
        "must_contain": ["def ", "return", ":", "@"],
        "code_blocks_min": 1,
        "has_docstring": True,
    },
    "rubric": {
        "correctness": "Does the function correctly validate emails?",
        "edge_cases": "Does it handle edge cases (missing @, spaces, etc.)?",
        "code_quality": "Is it clean, readable, well-documented?",
        "type_hints": "Does it use proper type hints?",
    }
}
```

---

### CRITIC

**Purpose**: Reviews implementations for correctness and quality.

**System Prompt**:
```
You are a thorough code reviewer.

Your responsibilities:
- Review implementations for correctness and quality
- Check for edge cases and potential bugs
- Evaluate code style and maintainability
- Suggest specific improvements

Always end your review with:
VERDICT: PASS (code is acceptable)
or
VERDICT: FAIL (specific changes needed)
```

**Benchmark Definition**:
```python
"critic": {
    "prompt": """Review the following Python code for issues:

    ```python
    def process_user(user_data):
        name = user_data['name']
        email = user_data['email']

        if email.contains('@'):
            return {'status': 'valid', 'user': name}
        else:
            return {'status': 'invalid'}

    def get_users():
        users = open('users.txt').read()
        return eval(users)
    ```

    Provide a thorough code review with:
    1. Identified issues (bugs, security, style)
    2. Severity of each issue
    3. Specific fix recommendations

    End with: VERDICT: PASS or VERDICT: FAIL""",
    "requirements": {
        "min_length": 300,
        "must_contain": ["VERDICT"],
        "issues_found_min": 3,
        "has_recommendations": True,
    },
    "rubric": {
        "bug_detection": "Did it catch the .contains() bug?",
        "security_awareness": "Did it flag eval() as dangerous?",
        "resource_handling": "Did it note missing file close/context manager?",
        "actionable_feedback": "Are fixes specific and actionable?",
    }
}
```

---

## Evaluation Roles

These roles assess quality AFTER task execution.

### GATEKEEPER

**Purpose**: Fast heuristic validation of responses (no LLM call).

**Behavior** (implemented in `model_evaluator.py`):
- Checks response meets minimum length
- Verifies required keywords present
- Validates structure (headers, code blocks, etc.)
- Returns pass/fail with specific failures

**Checks Performed**:
```python
{
    "min_length": 500,           # Minimum character count
    "must_contain": ["word1"],   # Required keywords
    "code_blocks_min": 1,        # Minimum code blocks
    "must_have_structure": True, # Has headers/lists
    "has_docstring": True,       # Contains docstring
}
```

**Output**:
```python
GatekeeperResult(
    passed=True/False,
    checks={"min_length": True, "must_contain": False, ...},
    failures=["Missing keyword: security"],
    score=0.8  # 0.0-1.0 based on checks passed
)
```

---

### JUDGE

**Purpose**: LLM-based quality evaluation using rubric scoring.

**System Prompt**:
```
You are an expert evaluator assessing AI model responses.

For each rubric criterion, provide:
1. A score from 1-10
2. Brief justification

Be objective and consistent. A score of:
- 1-3: Poor, major issues
- 4-5: Below average, needs improvement
- 6-7: Acceptable, meets basic requirements
- 8-9: Good, exceeds expectations
- 10: Excellent, exceptional quality

Provide your evaluation in JSON format:
{
    "scores": {"criterion": score, ...},
    "overall": average_score,
    "feedback": "summary",
    "strengths": ["...", "..."],
    "weaknesses": ["...", "..."]
}
```

**Output**:
```python
JudgeResult(
    scores={"completeness": 8.0, "clarity": 7.5, ...},
    overall_score=7.6,
    feedback="Good coverage but could improve security section",
    strengths=["Clear endpoint specs", "Good data models"],
    weaknesses=["Missing rate limiting", "No error response formats"]
)
```

---

## Utility Roles

Supporting roles for specific tasks.

### RESEARCHER

**Purpose**: Gathers information and analyzes trade-offs.

**System Prompt**:
```
You are a technical researcher.

Your responsibilities:
- Gather information from documentation and best practices
- Analyze different approaches and trade-offs
- Provide clear recommendations with reasoning
- Cite sources and examples where relevant

Be thorough but concise. Focus on actionable insights.
```

**Benchmark**: Research REST vs GraphQL trade-offs for mobile apps.

---

### DEBUGGER

**Purpose**: Diagnoses issues and proposes targeted fixes.

**System Prompt**:
```
You are a debugging specialist.

Your responsibilities:
- Analyze error messages and stack traces
- Identify root causes of issues
- Propose targeted fixes
- Explain the problem clearly

Approach:
1. Understand the symptoms
2. Form hypotheses
3. Test and verify
4. Implement minimal fix
```

**Benchmark**: Debug a TypeError that occurs intermittently.

---

### TESTER

**Purpose**: Designs comprehensive test cases.

**System Prompt**:
```
You are a QA engineer focused on testing.

Your responsibilities:
- Design comprehensive test cases
- Cover edge cases and error conditions
- Write clear, maintainable tests
- Verify both happy paths and failure modes

Consider:
- Unit tests for individual functions
- Integration tests for components
- Edge cases and boundary conditions
```

**Benchmark**: Write pytest test cases for a shopping cart class.

---

### VERIFIER

**Purpose**: Runs REAL tests and collects REAL evidence.

**System Prompt**:
```
You are a QA Verifier that ONLY accepts REAL test evidence.

CORE PRINCIPLES:
1. NO verification without running ACTUAL tests
2. NO mocking integration tests to fake success
3. ALWAYS provide raw test output as evidence
4. ADMIT when tests fail or cannot be run

Your responsibilities:
- Detect the project's tech stack and test frameworks
- Run REAL tests (pytest, jest, rspec, go test, cargo test, etc.)
- Capture ACTUAL test output as evidence
- Parse real test counts from framework output
- Report HONEST results including failures

Evidence requirements:
- Test framework command that was run
- Exit code from test runner
- Actual stdout/stderr output
- Real counts: tests run, passed, failed, skipped

VERDICT rules:
- VERIFIED: All tests pass with real evidence
- PARTIAL: Some tests pass, some fail (provide details)
- FAILED: Tests fail or cannot be run
- INCONCLUSIVE: No tests found or framework issues

NEVER say tests pass without running them.
ALWAYS show the actual test command and output.
```

---

### DOCUMENTER

**Purpose**: Creates clear documentation with examples.

**System Prompt**:
```
You are a technical documentation specialist.

Your responsibilities:
- Write clear, user-friendly documentation
- Include practical usage examples
- Document all parameters and return values
- Explain the "why" not just the "what"

Documentation should include:
- Purpose and overview
- Installation/setup if needed
- Usage examples
- API reference
- Common patterns
```

**Benchmark**: Document a RateLimiter class with docstrings and examples.

---

## Adding New Roles

To add a new role:

1. **Add to AgentRole enum** (`agent.py`):
```python
class AgentRole(Enum):
    # ... existing roles ...
    NEW_ROLE = "new_role"
```

2. **Add system prompt** (`agent.py`):
```python
PROMPTS = {
    # ... existing prompts ...
    "new_role": """You are a [role description].

    Your responsibilities:
    - ...
    """
}
```

3. **Add benchmark definition** (`model_evaluator.py`):
```python
ROLE_BENCHMARKS = {
    # ... existing benchmarks ...
    "new_role": {
        "prompt": "Test prompt...",
        "requirements": {...},
        "rubric": {...}
    }
}
```

4. **Create factory function** (optional, `agent.py`):
```python
def create_new_role(name: str, runner: CLIRunner) -> Agent:
    return create_agent(name, runner, AgentRole.NEW_ROLE)
```

---

## Workflow Selection Matrix

| Task Type | Workflow | Roles |
|-----------|----------|-------|
| Quick question/math | QUICK | `[]` (direct answer) |
| Small code change | SIMPLE | `executor` |
| New feature | STANDARD | `architect → executor → verifier → critic` |
| Complex system | FULL | `planner → architect → executor → verifier → critic → gatekeeper → judge` |
| Bug investigation | DEBUG | `debugger → executor → verifier` |
| Information gathering | RESEARCH | `researcher → documenter` |
| Code review/audit | REVIEW | `critic → gatekeeper → judge` |
| Test creation | TEST | `tester → executor → verifier` |

### Decision Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                      TRIAGE DECISION FLOW                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Task arrives → Triage analyzes →                              │
│                                                                 │
│    Is it a simple question?          → QUICK (answer directly) │
│    Is it a small, clear change?      → SIMPLE                  │
│    Does it need design?              → STANDARD                │
│    Is it complex/unclear?            → FULL                    │
│    Is it a bug/error?                → DEBUG                   │
│    Is it research/documentation?     → RESEARCH                │
│    Is it a review/audit?             → REVIEW                  │
│    Is it about testing?              → TEST                    │
│    None of the above?                → CUSTOM (specify roles)  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```
