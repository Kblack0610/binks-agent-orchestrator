"""
Agnostic Agent - Composable agent wrapper for ANY CLI runner.

This module provides a universal Agent class that:
  - Works with ANY CLIRunner implementation (Claude, Gemini, Ollama, etc.)
  - Separates concerns: Role is semantic, Runner is the backend
  - Enables easy backend swapping via with_runner()
  - Maps directly to Agno's Agent API for future migration

Architecture:
    Runner (How to talk to CLIs) → Agent (Role + Runner + Prompt) → Response

Migration to Agno:
    Agent(runner=ClaudeRunner(), ...)  →  agno.Agent(model=Claude(), ...)
    agent.invoke(prompt)               →  agent.run(prompt)
"""
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Any, Callable, Union
from enum import Enum

# Import runner base - works with ANY CLIRunner implementation
try:
    from .runners.base import CLIRunner, RunnerResult
except ImportError:
    from runners.base import CLIRunner, RunnerResult


class AgentRole(Enum):
    """
    Semantic roles for agents.

    These are purely for organization and filtering - the actual behavior
    is defined by the system_prompt, not the role.

    See docs/ROLES.md for comprehensive role documentation.
    """
    # Workflow Roles
    TRIAGE = "triage"        # Entry point: routes simple vs complex tasks
    PLANNER = "planner"      # Requirements gathering and clarification
    ARCHITECT = "architect"  # Solution design
    EXECUTOR = "executor"    # Implementation
    IMPLEMENTER = "implementer"  # Alias for executor
    CRITIC = "critic"        # Code review
    REVIEWER = "reviewer"    # Alias for critic

    # Evaluation Roles
    GATEKEEPER = "gatekeeper"  # Fast heuristic validation
    JUDGE = "judge"            # LLM-based quality scoring

    # Utility Roles
    RESEARCHER = "researcher"
    TESTER = "tester"
    VERIFIER = "verifier"    # QA agent that runs REAL tests
    DOCUMENTER = "documenter"
    DEBUGGER = "debugger"
    CUSTOM = "custom"


# =============================================================================
# WORKFLOWS - Predefined role sequences for common task patterns
# =============================================================================
# Each workflow is a list of role names that will be executed in order.
# Triage uses these to route tasks to the appropriate workflow.
# =============================================================================

WORKFLOWS = {
    # Direct answer - no agents needed
    "quick": {
        "description": "Direct answer, no agent workflow needed",
        "roles": [],
        "use_when": "Simple questions, math, definitions, one-line answers",
        "examples": ["What is 2+2?", "Convert 5km to miles", "What does 'async' mean?"]
    },

    # Simple implementation - just execute
    "simple": {
        "description": "Quick implementation without design phase",
        "roles": ["executor"],
        "use_when": "Small, well-defined tasks with clear requirements",
        "examples": ["Add a print statement", "Rename this variable", "Fix this typo"]
    },

    # Standard development - design, implement, test, review
    "standard": {
        "description": "Standard development workflow with design, testing, and review",
        "roles": ["architect", "executor", "verifier", "critic"],
        "use_when": "Features that need design but aren't massive",
        "examples": ["Add a new API endpoint", "Implement a caching layer", "Create a config system"]
    },

    # Full workflow - planning through evaluation
    "full": {
        "description": "Complete workflow with planning, implementation, testing, and evaluation",
        "roles": ["planner", "architect", "executor", "verifier", "critic", "gatekeeper", "judge"],
        "use_when": "Complex features, unclear requirements, critical systems",
        "examples": ["Build authentication system", "Design a plugin architecture", "Implement payment processing"]
    },

    # Debug workflow - diagnose and fix
    "debug": {
        "description": "Debugging workflow for investigating and fixing issues",
        "roles": ["debugger", "executor", "verifier"],
        "use_when": "Bug reports, errors, unexpected behavior",
        "examples": ["Fix this stack trace", "Why is this test failing?", "Debug memory leak"]
    },

    # Research workflow - investigate and document
    "research": {
        "description": "Research and documentation workflow",
        "roles": ["researcher", "documenter"],
        "use_when": "Information gathering, documentation, analysis",
        "examples": ["How does X library work?", "Document this codebase", "Compare these approaches"]
    },

    # Review workflow - critic + evaluation
    "review": {
        "description": "Code review and quality assessment",
        "roles": ["critic", "gatekeeper", "judge"],
        "use_when": "PR reviews, code audits, quality checks",
        "examples": ["Review this PR", "Audit security of this module", "Assess code quality"]
    },

    # Test workflow - create and run tests
    "test": {
        "description": "Testing workflow for creating and verifying tests",
        "roles": ["tester", "executor", "verifier"],
        "use_when": "Test creation, test fixes, coverage improvements",
        "examples": ["Write tests for this function", "Fix failing tests", "Improve test coverage"]
    },
}

# Role descriptions for triage to understand what each role does
ROLE_DESCRIPTIONS = {
    "planner": "Gathers requirements, asks clarifying questions, defines specifications",
    "architect": "Designs solutions, defines structure, plans implementation approach",
    "executor": "Implements code, writes the actual solution",
    "critic": "Reviews code for quality, bugs, and improvements",
    "gatekeeper": "Fast validation checks - syntax, requirements met, basic quality",
    "judge": "Deep quality scoring against rubric criteria",
    "researcher": "Gathers information, investigates libraries, explores options",
    "debugger": "Diagnoses issues, traces errors, identifies root causes",
    "tester": "Writes test cases, designs test strategies",
    "verifier": "Runs actual tests, validates implementations work",
    "documenter": "Writes documentation, explains code, creates guides",
}


@dataclass
class AgentResponse:
    """
    Standardized response from any agent.

    Attributes:
        content: The main response text
        success: Whether the invocation succeeded
        verdict: Optional verdict (PASS/FAIL) for review agents
        artifacts: List of file paths or artifacts created
        metadata: Additional metadata from the runner
    """
    content: str
    success: bool = True
    verdict: Optional[str] = None  # "PASS", "FAIL", "NEEDS_REVISION"
    artifacts: List[str] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)

    @property
    def passed(self) -> bool:
        """Check if verdict is PASS."""
        return self.verdict == "PASS" if self.verdict else False

    @property
    def failed(self) -> bool:
        """Check if verdict is FAIL."""
        return self.verdict == "FAIL" if self.verdict else False

    def __str__(self) -> str:
        return self.content


class Agent:
    """
    Fully agnostic agent that wraps ANY CLIRunner.

    The same Agent class can be an architect, executor, critic, etc.
    based on the system_prompt and role configuration.

    This design ensures:
    1. ONE Agent class - no separate ArchitectAgent, ExecutorAgent, etc.
    2. Runner is a parameter - same Agent can use Claude, Gemini, Ollama, etc.
    3. Role is semantic only - for organization/filtering, not behavior
    4. System prompt defines behavior - fully customizable
    5. Agno-ready - maps directly to agno.Agent(model=..., instructions=...)

    Usage:
        # Create agents with different backends
        architect = Agent(
            name="architect",
            role=AgentRole.ARCHITECT,
            runner=GeminiRunner(),  # or ClaudeRunner(), OllamaRunner(), etc.
            system_prompt="You are a senior software architect..."
        )

        executor = Agent(
            name="executor",
            role=AgentRole.EXECUTOR,
            runner=ClaudeRunner(),  # or any other runner
            system_prompt="You implement solutions based on plans..."
        )

        # Invoke with any prompt
        response = architect.invoke("Design a REST API")
        response = executor.invoke("Implement the design", context=response.content)

        # Easy backend swapping
        architect_v2 = architect.with_runner(ClaudeRunner())
    """

    def __init__(
        self,
        name: str,
        runner: CLIRunner,
        role: AgentRole = AgentRole.CUSTOM,
        system_prompt: str = "",
        response_parser: Optional[Callable[[str, "RunnerResult"], "AgentResponse"]] = None,
        debug: bool = False
    ):
        """
        Create an Agent.

        Args:
            name: Identifier for the agent
            runner: Any CLIRunner implementation
            role: Semantic role (for organization)
            system_prompt: Instructions defining agent behavior
            response_parser: Custom parser for responses
            debug: Enable debug output
        """
        self.name = name
        self.runner = runner
        self.role = role
        self.system_prompt = system_prompt
        self.response_parser = response_parser or self._default_parser
        self.debug = debug

    def invoke(
        self,
        prompt: str,
        context: str = "",
        **kwargs
    ) -> AgentResponse:
        """
        Invoke the agent with a prompt and optional context.

        Args:
            prompt: The task/question for the agent
            context: Optional context (e.g., from previous agents)
            **kwargs: Additional arguments passed to the runner

        Returns:
            AgentResponse with content and metadata
        """
        # Build full prompt
        parts = []

        if self.system_prompt:
            parts.append(self.system_prompt)

        if context:
            parts.append(f"Context:\n{context}")

        parts.append(f"Task:\n{prompt}")

        full_prompt = "\n\n".join(parts)

        if self.debug:
            print(f"[Agent:{self.name}] Invoking with {len(full_prompt)} chars")

        # Run through the backend
        result = self.runner.run(full_prompt, **kwargs)

        if self.debug:
            print(f"[Agent:{self.name}] Got response: {len(result.content)} chars")

        # Parse response
        return self.response_parser(result.content, result)

    def _default_parser(
        self,
        content: str,
        result: Optional[RunnerResult] = None
    ) -> AgentResponse:
        """
        Default response parser.

        Extracts verdict (PASS/FAIL) if present in the response.
        """
        verdict = None

        # Check for explicit verdict markers (flexible matching)
        content_upper = content.upper()
        # Check last 100 chars for verdict (often at the end)
        tail = content_upper[-100:] if len(content_upper) > 100 else content_upper

        if "VERDICT: PASS" in content_upper or "VERDICT:PASS" in content_upper:
            verdict = "PASS"
        elif "VERDICT: FAIL" in content_upper or "VERDICT:FAIL" in content_upper:
            verdict = "FAIL"
        # Also check for PASS/FAIL on its own line at the end
        elif tail.strip().endswith("PASS") or "\nPASS" in tail:
            verdict = "PASS"
        elif tail.strip().endswith("FAIL") or "\nFAIL" in tail:
            verdict = "FAIL"
        elif "NEEDS_REVISION" in content_upper:
            verdict = "NEEDS_REVISION"

        return AgentResponse(
            content=content,
            success=result.success if result else True,
            verdict=verdict,
            metadata={
                "backend": self.runner.name,
                "role": self.role.value,
                "agent": self.name,
                "model": result.model if result else "",
            }
        )

    def with_runner(self, new_runner: CLIRunner) -> "Agent":
        """
        Create a copy of this agent with a different runner.

        This enables easy backend swapping:
            gemini_architect = architect.with_runner(GeminiRunner())
            claude_architect = architect.with_runner(ClaudeRunner())

        Args:
            new_runner: The new runner to use

        Returns:
            New Agent with same config but different runner
        """
        return Agent(
            name=self.name,
            runner=new_runner,
            role=self.role,
            system_prompt=self.system_prompt,
            response_parser=self.response_parser,
            debug=self.debug
        )

    def with_prompt(self, new_prompt: str) -> "Agent":
        """
        Create a copy of this agent with a different system prompt.

        Args:
            new_prompt: The new system prompt

        Returns:
            New Agent with same config but different prompt
        """
        return Agent(
            name=self.name,
            runner=self.runner,
            role=self.role,
            system_prompt=new_prompt,
            response_parser=self.response_parser,
            debug=self.debug
        )

    def __repr__(self) -> str:
        return f"Agent(name='{self.name}', role={self.role.value}, runner={self.runner.name})"


# =============================================================================
# Pre-defined System Prompts
# =============================================================================

PROMPTS = {
    "architect": """You are a senior software architect.

Your responsibilities:
- Design solutions with clear component boundaries
- Break down complex tasks into actionable steps
- Identify dependencies and integration points
- Consider scalability, maintainability, and security

When reviewing code:
- Evaluate architecture and design patterns
- Check for potential issues and improvements
- End with a clear VERDICT: PASS or VERDICT: FAIL""",

    "executor": """You are an expert software implementer.

Your responsibilities:
- Write clean, production-ready code
- Follow established patterns and conventions
- Include appropriate error handling
- Write self-documenting code with clear naming

Focus on:
- Correctness first, optimization second
- Consistent style with the existing codebase
- Minimal, focused changes that accomplish the task

OUTPUT FORMAT (CRITICAL):
- Output your code directly in markdown code blocks
- Do NOT ask for permission or confirmation
- Do NOT describe what the code does - just output the code
- Start your response with the code block immediately
- Use brief inline comments only where logic isn't obvious

Example response:
```python
def example_function(param):
    result = do_something(param)
    return result
```""",

    "critic": """You are a thorough code reviewer.

Your responsibilities:
- Review implementations for correctness and quality
- Check for edge cases and potential bugs
- Evaluate code style and maintainability
- Suggest specific improvements

Always end your review with:
VERDICT: PASS (code is acceptable)
or
VERDICT: FAIL (specific changes needed)""",

    "researcher": """You are a technical researcher.

Your responsibilities:
- Gather information from documentation and best practices
- Analyze different approaches and trade-offs
- Provide clear recommendations with reasoning
- Cite sources and examples where relevant

Be thorough but concise. Focus on actionable insights.""",

    "tester": """You are a QA engineer focused on testing.

Your responsibilities:
- Design comprehensive test cases
- Cover edge cases and error conditions
- Write clear, maintainable tests
- Verify both happy paths and failure modes

Consider:
- Unit tests for individual functions
- Integration tests for components
- Edge cases and boundary conditions""",

    "debugger": """You are a debugging specialist.

Your responsibilities:
- Analyze error messages and stack traces
- Identify root causes of issues
- Propose targeted fixes
- Explain the problem clearly

Approach:
1. Understand the symptoms
2. Form hypotheses
3. Test and verify
4. Implement minimal fix""",

    "verifier": """You are a QA Verifier that ONLY accepts REAL test evidence.

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
ALWAYS show the actual test command and output.""",

    "triage": """You are a workflow router. Given a task, select the appropriate workflow.

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
  Examples: "What is 2+2?", "Convert 5km to miles", "What does 'async' mean?"

- SIMPLE: executor only. Small, well-defined tasks with clear requirements.
  Examples: "Add a print statement", "Rename this variable", "Fix this typo"

- STANDARD: architect → executor → verifier → critic. Features that need design but aren't massive.
  Examples: "Add a new API endpoint", "Implement a caching layer", "Create a config system"

- FULL: planner → architect → executor → verifier → critic → gatekeeper → judge. Complex features, unclear requirements.
  Examples: "Build authentication system", "Design a plugin architecture", "Implement payment processing"

- DEBUG: debugger → executor → verifier. Bug reports, errors, unexpected behavior.
  Examples: "Fix this stack trace", "Why is this test failing?", "Debug memory leak"

- RESEARCH: researcher → documenter. Information gathering, documentation, analysis.
  Examples: "How does X library work?", "Document this codebase", "Compare these approaches"

- REVIEW: critic → gatekeeper → judge. PR reviews, code audits, quality checks.
  Examples: "Review this PR", "Audit security of this module", "Assess code quality"

- TEST: tester → executor → verifier. Test creation, test fixes, coverage improvements.
  Examples: "Write tests for this function", "Fix failing tests", "Improve test coverage"

YOUR RESPONSE FORMAT:
You MUST respond with a JSON object:

{
  "workflow": "<WORKFLOW_NAME or CUSTOM>",
  "roles": ["role1", "role2", ...],
  "reasoning": "Brief explanation of why this workflow fits the task",
  "answer": "If QUICK workflow, provide the direct answer here. Otherwise null."
}

RULES:
1. For predefined workflows, set "workflow" to the name and "roles" to its role sequence
2. For custom workflows, set "workflow" to "CUSTOM" and specify your own "roles" array
3. For QUICK workflow, "roles" should be [] and provide "answer" directly
4. Always explain your reasoning
5. When in doubt, prefer simpler workflows - don't over-engineer""",

    "planner": """You are a requirements analyst and planner.

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

Be thorough but practical. Focus on what's needed to proceed, not perfection.""",

    "gatekeeper": """You are a quality gatekeeper performing fast validation.

Your responsibilities:
- Check responses meet minimum requirements
- Verify required elements are present
- Validate structure and format
- Provide quick pass/fail with specific failures

Check for:
- Minimum length/content
- Required keywords or concepts
- Proper structure (headers, code blocks, etc.)
- Completeness of the response

Output format:
GATE: PASS or GATE: FAIL
Checks: [list of checks performed]
Failures: [list of specific failures if any]""",

    "judge": """You are an expert evaluator assessing response quality.

For each rubric criterion, provide:
1. A score from 1-10
2. Brief justification

Be objective and consistent. A score of:
- 1-3: Poor, major issues
- 4-5: Below average, needs improvement
- 6-7: Acceptable, meets basic requirements
- 8-9: Good, exceeds expectations
- 10: Excellent, exceptional quality

Provide your evaluation as:
SCORES:
- [criterion]: [score]/10 - [justification]
OVERALL: [average]/10
STRENGTHS: [list strengths]
WEAKNESSES: [list weaknesses]
SUMMARY: [brief overall assessment]""",
}


# =============================================================================
# Factory Function
# =============================================================================

def create_agent(
    name: str,
    runner: CLIRunner,
    role: AgentRole = AgentRole.CUSTOM,
    system_prompt: Optional[str] = None,
    debug: bool = False
) -> Agent:
    """
    Factory function to create agents with sensible defaults.

    If no system_prompt provided, uses PROMPTS[role.value] if available.

    Args:
        name: Agent identifier
        runner: Any CLIRunner implementation
        role: Semantic role (default: CUSTOM)
        system_prompt: Custom prompt (default: uses PROMPTS based on role)
        debug: Enable debug output

    Returns:
        Configured Agent instance

    Example:
        # Uses default architect prompt
        architect = create_agent("arch", GeminiRunner(), AgentRole.ARCHITECT)

        # Uses custom prompt
        custom = create_agent("custom", ClaudeRunner(), system_prompt="You are...")
    """
    if system_prompt is None:
        system_prompt = PROMPTS.get(role.value, "")

    return Agent(
        name=name,
        runner=runner,
        role=role,
        system_prompt=system_prompt,
        debug=debug
    )


# =============================================================================
# Convenience Creators
# =============================================================================

def create_architect(runner: CLIRunner, name: str = "architect", **kwargs) -> Agent:
    """Create an architect agent with default prompt."""
    return create_agent(name, runner, AgentRole.ARCHITECT, **kwargs)


def create_executor(runner: CLIRunner, name: str = "executor", **kwargs) -> Agent:
    """Create an executor agent with default prompt."""
    return create_agent(name, runner, AgentRole.EXECUTOR, **kwargs)


def create_critic(runner: CLIRunner, name: str = "critic", **kwargs) -> Agent:
    """Create a critic agent with default prompt."""
    return create_agent(name, runner, AgentRole.CRITIC, **kwargs)


def create_researcher(runner: CLIRunner, name: str = "researcher", **kwargs) -> Agent:
    """Create a researcher agent with default prompt."""
    return create_agent(name, runner, AgentRole.RESEARCHER, **kwargs)


def create_verifier(runner: CLIRunner, name: str = "verifier", **kwargs) -> Agent:
    """Create a verifier agent with default prompt for QA verification."""
    return create_agent(name, runner, AgentRole.VERIFIER, **kwargs)


def create_triage(runner: CLIRunner, name: str = "triage", **kwargs) -> Agent:
    """Create a triage agent that routes simple vs complex tasks."""
    return create_agent(name, runner, AgentRole.TRIAGE, **kwargs)


def create_planner(runner: CLIRunner, name: str = "planner", **kwargs) -> Agent:
    """Create a planner agent for requirements gathering."""
    return create_agent(name, runner, AgentRole.PLANNER, **kwargs)


def create_gatekeeper(runner: CLIRunner, name: str = "gatekeeper", **kwargs) -> Agent:
    """Create a gatekeeper agent for fast validation."""
    return create_agent(name, runner, AgentRole.GATEKEEPER, **kwargs)


def create_judge(runner: CLIRunner, name: str = "judge", **kwargs) -> Agent:
    """Create a judge agent for LLM-based quality scoring."""
    return create_agent(name, runner, AgentRole.JUDGE, **kwargs)


# =============================================================================
# Standalone Usage Example
# =============================================================================

if __name__ == "__main__":
    # This example shows the API without actually running anything
    print("Agent Module - Agnostic CLI Agent Wrapper")
    print("=" * 50)
    print()
    print("Available roles:")
    for role in AgentRole:
        print(f"  - {role.value}")
    print()
    print("Available default prompts:")
    for name in PROMPTS:
        print(f"  - {name}")
    print()
    print("Example usage:")
    print("""
    from runners import ClaudeRunner, GeminiRunner
    from agent import create_agent, AgentRole

    # Create agents with different backends
    architect = create_agent("arch", GeminiRunner(), AgentRole.ARCHITECT)
    executor = create_agent("impl", ClaudeRunner(), AgentRole.EXECUTOR)

    # Invoke
    plan = architect.invoke("Design a REST API for user management")
    code = executor.invoke("Implement the design", context=plan.content)

    # Easy backend swapping
    alt_architect = architect.with_runner(ClaudeRunner())
    """)
