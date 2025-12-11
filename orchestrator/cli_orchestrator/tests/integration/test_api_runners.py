"""
REAL Integration Tests for API-based Runners (Groq, OpenRouter)

These tests ACTUALLY call the APIs. No mocks.
Run with: pytest -m requires_api tests/integration/test_api_runners.py -v

Requires environment variables:
- GROQ_API_KEY for Groq tests
- OPENROUTER_API_KEY for OpenRouter tests
"""
import os
import sys
import pytest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from runners import GroqRunner, OpenRouterRunner
from agent import Agent, AgentRole, create_agent


# =============================================================================
# Fixtures
# =============================================================================

def _check_groq():
    """Check if Groq API is available."""
    try:
        return GroqRunner().is_available()
    except:
        return False


def _check_openrouter():
    """Check if OpenRouter API is available."""
    try:
        return OpenRouterRunner().is_available()
    except:
        return False


@pytest.fixture
def groq_runner():
    """Returns Groq runner if API key is set."""
    if not _check_groq():
        pytest.skip("GROQ_API_KEY not set or invalid")
    return GroqRunner(debug=True)


@pytest.fixture
def openrouter_runner():
    """Returns OpenRouter runner if API key is set."""
    if not _check_openrouter():
        pytest.skip("OPENROUTER_API_KEY not set or invalid")
    return OpenRouterRunner(debug=True)


# =============================================================================
# Groq Tests
# =============================================================================

@pytest.mark.requires_api
class TestGroqRunner:
    """Tests for Groq API runner."""

    def test_groq_responds(self, groq_runner):
        """Does Groq API return a response?"""
        result = groq_runner.run("Say 'hello' in one word")

        assert result.success, f"Groq failed: {result.error}"
        assert result.content, "Empty response from Groq"
        assert result.backend == "groq"

    def test_groq_simple_math(self, groq_runner):
        """Can Groq do simple reasoning?"""
        result = groq_runner.run("What is 7 + 8? Reply with just the number.")

        assert result.success, f"Groq failed: {result.error}"
        assert "15" in result.content, f"Expected '15' in response: {result.content}"

    def test_groq_includes_metadata(self, groq_runner):
        """Does Groq return proper metadata?"""
        result = groq_runner.run("Say 'test'")

        assert result.success
        assert result.model, "Model not set"
        assert result.execution_time > 0, "Execution time not recorded"
        assert result.tokens_used is not None, "Token usage not tracked"

    def test_groq_as_agent(self, groq_runner):
        """Can Groq power an Agent?"""
        agent = create_agent("groq-test", groq_runner, AgentRole.EXECUTOR)

        response = agent.invoke("What is the capital of France? Reply in one word.")

        assert response.success
        assert response.content
        assert "paris" in response.content.lower()

    def test_groq_verdict_parsing(self, groq_runner):
        """Can Groq return parseable verdicts?"""
        agent = create_agent(
            name="groq-critic",
            runner=groq_runner,
            role=AgentRole.CRITIC
        )

        response = agent.invoke(
            "Is the statement '2 + 2 = 4' correct? "
            "Reply with VERDICT: PASS or VERDICT: FAIL"
        )

        assert response.success, f"Failed: {response.error}"
        assert response.verdict in ["PASS", "FAIL"], \
            f"Verdict not parsed from Groq! Response: {response.content[-100:]}"

    def test_groq_fast_response(self, groq_runner):
        """Groq should be fast (under 5 seconds for simple prompt)."""
        result = groq_runner.run("Reply with 'fast'")

        assert result.success
        assert result.execution_time < 5.0, \
            f"Groq took {result.execution_time:.2f}s - expected <5s"

    def test_groq_model_override(self, groq_runner):
        """Can override model per-request."""
        result = groq_runner.run(
            "Say 'test'",
            model="llama-3.1-8b-instant"
        )

        assert result.success
        assert result.model == "llama-3.1-8b-instant"


# =============================================================================
# OpenRouter Tests
# =============================================================================

@pytest.mark.requires_api
class TestOpenRouterRunner:
    """Tests for OpenRouter API runner."""

    def test_openrouter_responds(self, openrouter_runner):
        """Does OpenRouter API return a response?"""
        result = openrouter_runner.run("Say 'hello' in one word")

        assert result.success, f"OpenRouter failed: {result.error}"
        assert result.content, "Empty response from OpenRouter"
        assert result.backend == "openrouter"

    def test_openrouter_simple_math(self, openrouter_runner):
        """Can OpenRouter do simple reasoning?"""
        result = openrouter_runner.run("What is 9 + 6? Reply with just the number.")

        assert result.success, f"OpenRouter failed: {result.error}"
        assert "15" in result.content, f"Expected '15' in response: {result.content}"

    def test_openrouter_includes_metadata(self, openrouter_runner):
        """Does OpenRouter return proper metadata?"""
        result = openrouter_runner.run("Say 'test'")

        assert result.success
        assert result.model, "Model not set"
        assert result.execution_time > 0, "Execution time not recorded"

    def test_openrouter_as_agent(self, openrouter_runner):
        """Can OpenRouter power an Agent?"""
        agent = create_agent("openrouter-test", openrouter_runner, AgentRole.EXECUTOR)

        response = agent.invoke("What planet is known as the Red Planet? Reply in one word.")

        assert response.success
        assert response.content
        assert "mars" in response.content.lower()

    def test_openrouter_verdict_parsing(self, openrouter_runner):
        """Can OpenRouter return parseable verdicts?"""
        agent = create_agent(
            name="openrouter-critic",
            runner=openrouter_runner,
            role=AgentRole.CRITIC
        )

        response = agent.invoke(
            "Is the statement 'The Earth is flat' correct? "
            "Reply with VERDICT: PASS or VERDICT: FAIL"
        )

        assert response.success, f"Failed: {response.error}"
        assert response.verdict in ["PASS", "FAIL"], \
            f"Verdict not parsed from OpenRouter! Response: {response.content[-100:]}"

    def test_openrouter_free_model(self, openrouter_runner):
        """Can use free models."""
        # Use an explicitly free model (models change frequently)
        runner = OpenRouterRunner(
            model="nvidia/nemotron-nano-9b-v2:free",
            debug=True
        )

        if not runner.is_available():
            pytest.skip("OpenRouter not available")

        result = runner.run("Say 'free'")

        assert result.success, f"Free model failed: {result.error}"
        assert ":free" in result.model or "nvidia" in result.model.lower()


# =============================================================================
# Model Listing Tests (No API call required)
# =============================================================================

class TestModelListing:
    """Tests for model listing utilities."""

    def test_groq_list_models(self):
        """Groq lists available models."""
        models = GroqRunner.list_models()

        assert len(models) > 0
        assert "llama-3.3-70b-versatile" in models

    def test_openrouter_list_free_models(self):
        """OpenRouter lists free models."""
        models = OpenRouterRunner.list_free_models()

        assert len(models) > 0
        assert any("free" in m for m in models.keys())

    def test_openrouter_list_all_models(self):
        """OpenRouter lists all known models."""
        models = OpenRouterRunner.list_all_models()

        assert len(models) > len(OpenRouterRunner.list_free_models())


# =============================================================================
# Error Handling Tests
# =============================================================================

class TestErrorHandling:
    """Tests for error handling."""

    def test_groq_without_key(self):
        """Groq fails gracefully without API key."""
        runner = GroqRunner(api_key="")  # Invalid key

        assert not runner.is_available()

        result = runner.run("test")
        assert not result.success
        assert "not set" in result.error.lower() or "api" in result.error.lower()

    def test_openrouter_without_key(self):
        """OpenRouter fails gracefully without API key."""
        runner = OpenRouterRunner(api_key="")  # Invalid key

        assert not runner.is_available()

        result = runner.run("test")
        assert not result.success
        assert "not set" in result.error.lower() or "api" in result.error.lower()
