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
    """
    ARCHITECT = "architect"
    EXECUTOR = "executor"
    CRITIC = "critic"
    RESEARCHER = "researcher"
    TESTER = "tester"
    DOCUMENTER = "documenter"
    DEBUGGER = "debugger"
    CUSTOM = "custom"


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
- Minimal, focused changes that accomplish the task""",

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
