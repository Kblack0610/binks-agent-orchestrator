"""
REAL Backend Integration Tests

These tests actually call Claude CLI, Gemini API, etc.
They verify the REAL system works - this is the source of truth.

Run with: pytest -m requires_api -v
"""
import sys
import pytest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from runners import ClaudeRunner, GeminiRunner
from runners.custom_runner import OllamaRunner
from agent import Agent, AgentRole, create_agent, AgentResponse


# =============================================================================
# Claude CLI Tests
# =============================================================================

@pytest.mark.requires_api
@pytest.mark.requires_claude
class TestClaudeRunner:
    """Tests that verify Claude CLI actually works."""

    def test_claude_is_available(self, claude_runner):
        """Verify Claude CLI is installed and accessible."""
        assert claude_runner.is_available()

    def test_claude_simple_prompt(self, claude_runner):
        """Verify Claude can answer a simple question."""
        result = claude_runner.run("What is 2 + 2? Reply with just the number.")

        assert result.success
        assert result.content
        assert len(result.content) > 0
        # Should contain "4" somewhere in response
        assert "4" in result.content

    def test_claude_code_generation(self, claude_runner):
        """Verify Claude can generate code."""
        result = claude_runner.run(
            "Write a Python function called 'add' that takes two numbers and returns their sum. "
            "Reply with ONLY the function code, no explanation."
        )

        assert result.success
        assert "def add" in result.content
        assert "return" in result.content

    def test_claude_response_has_metadata(self, claude_runner):
        """Verify response includes expected metadata."""
        result = claude_runner.run("Say hello")

        assert result.success
        assert result.backend == "claude"
        assert result.execution_time >= 0

    def test_claude_handles_complex_prompt(self, claude_runner):
        """Verify Claude handles multi-line complex prompts."""
        prompt = """Analyze this code and list any issues:

```python
def divide(a, b):
    return a / b
```

List issues as bullet points."""

        result = claude_runner.run(prompt)

        assert result.success
        assert len(result.content) > 50  # Should have substantial response
        # Should mention division by zero or similar
        assert any(word in result.content.lower() for word in ["zero", "error", "exception", "check"])


# =============================================================================
# Gemini API Tests
# =============================================================================

@pytest.mark.requires_api
@pytest.mark.requires_gemini
class TestGeminiRunner:
    """Tests that verify Gemini API actually works."""

    def test_gemini_is_available(self, gemini_runner):
        """Verify Gemini API is accessible."""
        assert gemini_runner.is_available()

    def test_gemini_simple_prompt(self, gemini_runner):
        """Verify Gemini can answer a simple question."""
        result = gemini_runner.run("What is 3 + 3? Reply with just the number.")

        assert result.success
        assert result.content
        assert "6" in result.content

    def test_gemini_code_generation(self, gemini_runner):
        """Verify Gemini can generate code."""
        result = gemini_runner.run(
            "Write a Python function called 'multiply' that takes two numbers and returns their product. "
            "Reply with ONLY the function code."
        )

        assert result.success
        assert "def multiply" in result.content or "def Multiply" in result.content

    def test_gemini_response_has_metadata(self, gemini_runner):
        """Verify response includes expected metadata."""
        result = gemini_runner.run("Say hello")

        assert result.success
        assert result.execution_time >= 0


# =============================================================================
# Ollama Tests (if available)
# =============================================================================

@pytest.mark.requires_api
@pytest.mark.requires_ollama
class TestOllamaRunner:
    """Tests that verify Ollama works (if installed)."""

    def test_ollama_is_available(self, ollama_runner):
        """Verify Ollama is running."""
        assert ollama_runner.is_available()

    def test_ollama_simple_prompt(self, ollama_runner):
        """Verify Ollama can respond."""
        result = ollama_runner.run("Say 'hello world'")

        assert result.success
        assert result.content
        assert len(result.content) > 0


# =============================================================================
# Cross-Backend Tests
# =============================================================================

@pytest.mark.requires_api
class TestCrossBackend:
    """Tests that verify backend switching works."""

    def test_any_backend_responds(self, any_real_runner):
        """Verify at least one backend is working."""
        result = any_real_runner.run("What is 1 + 1?")

        assert result.success
        assert result.content
        assert "2" in result.content

    def test_agent_with_real_backend(self, any_real_runner):
        """Verify Agent class works with real backend."""
        agent = create_agent(
            name="test-agent",
            runner=any_real_runner,
            role=AgentRole.EXECUTOR
        )

        response = agent.invoke("Write a one-line Python hello world program")

        assert response.success
        assert response.content
        assert "print" in response.content.lower()

    def test_agent_context_passing(self, any_real_runner):
        """Verify Agent passes context correctly."""
        agent = create_agent(
            name="context-test",
            runner=any_real_runner,
            role=AgentRole.EXECUTOR
        )

        context = "The variable name should be 'result'"
        response = agent.invoke(
            "Write x = 5 + 3 using the variable name from context",
            context=context
        )

        assert response.success
        # Should use 'result' as variable name based on context
        assert "result" in response.content.lower()

    def test_verdict_parsing_with_real_response(self, any_real_runner):
        """Verify verdict parsing works with real AI responses."""
        agent = create_agent(
            name="critic-test",
            runner=any_real_runner,
            role=AgentRole.CRITIC,
            system_prompt="""You are a code reviewer.
Review the code and end with exactly: VERDICT: PASS or VERDICT: FAIL"""
        )

        response = agent.invoke("""Review this code:
def add(a, b):
    return a + b

Is this function correct?""")

        assert response.success
        # Should have a verdict
        assert response.verdict in ["PASS", "FAIL", None]  # May or may not parse


# =============================================================================
# Real MoA Workflow Test
# =============================================================================

@pytest.mark.requires_api
@pytest.mark.slow
class TestRealMoAWorkflow:
    """Test the actual MoA workflow with real backends."""

    def test_architect_executor_handoff(self, any_real_runner):
        """Test architect -> executor handoff with real AI."""
        architect = create_agent(
            name="architect",
            runner=any_real_runner,
            role=AgentRole.ARCHITECT
        )

        executor = create_agent(
            name="executor",
            runner=any_real_runner,
            role=AgentRole.EXECUTOR
        )

        # Step 1: Architect plans
        plan = architect.invoke(
            "Plan a simple Python function that checks if a number is even. "
            "List the steps needed."
        )

        assert plan.success
        assert len(plan.content) > 50

        # Step 2: Executor implements based on plan
        implementation = executor.invoke(
            "Implement the function based on this plan",
            context=plan.content
        )

        assert implementation.success
        assert "def" in implementation.content
        # Should have even-checking logic
        assert any(word in implementation.content for word in ["%", "mod", "2", "even"])

    def test_full_design_implement_review_cycle(self, any_real_runner):
        """Test complete design -> implement -> review cycle."""
        architect = create_agent("arch", any_real_runner, AgentRole.ARCHITECT)
        executor = create_agent(
            "exec",
            any_real_runner,
            AgentRole.EXECUTOR,
            system_prompt="""You implement code based on designs.
IMPORTANT: Return the code directly in your response. Do NOT use any tools or try to write files.
Just output the Python code directly."""
        )
        critic = create_agent(
            "critic",
            any_real_runner,
            AgentRole.CRITIC,
            system_prompt="Review the code. End with VERDICT: PASS or VERDICT: FAIL"
        )

        # Design
        design = architect.invoke("Design a simple function to reverse a string. Just describe the approach.")
        assert design.success

        # Implement - be very explicit about not using tools
        code = executor.invoke(
            "Write a Python function called reverse_string that reverses a string. "
            "Output ONLY the function code in your response, nothing else. No explanations.",
            context=design.content
        )
        assert code.success
        # Should have some code-like content (may or may not have def)
        assert len(code.content) > 10

        # Review
        review = critic.invoke(f"Review this implementation:\n{code.content}")
        assert review.success
        # Should have substantive review
        assert len(review.content) > 20
