"""
Unit tests for Agent class.

These tests use mocks for speed - verify with real backends using integration tests.
"""
import sys
import pytest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from agent import Agent, AgentRole, AgentResponse, create_agent, PROMPTS
from tests.mocks.mock_runners import MockRunner, PassingCriticRunner, FailingCriticRunner


class TestAgentResponse:
    """Tests for AgentResponse dataclass."""

    def test_response_creation(self):
        """Test basic response creation."""
        response = AgentResponse(content="Test content")
        assert response.content == "Test content"
        assert response.success is True
        assert response.verdict is None

    def test_passed_property(self):
        """Test passed property."""
        pass_response = AgentResponse(content="", verdict="PASS")
        fail_response = AgentResponse(content="", verdict="FAIL")
        no_verdict = AgentResponse(content="")

        assert pass_response.passed is True
        assert fail_response.passed is False
        assert no_verdict.passed is False

    def test_failed_property(self):
        """Test failed property."""
        pass_response = AgentResponse(content="", verdict="PASS")
        fail_response = AgentResponse(content="", verdict="FAIL")

        assert pass_response.failed is False
        assert fail_response.failed is True

    def test_str_representation(self):
        """Test string representation."""
        response = AgentResponse(content="Hello world")
        assert str(response) == "Hello world"


class TestAgentRole:
    """Tests for AgentRole enum."""

    def test_all_roles_exist(self):
        """Verify all expected roles exist."""
        expected = ["ARCHITECT", "EXECUTOR", "CRITIC", "RESEARCHER", "TESTER", "DEBUGGER", "CUSTOM"]
        for role_name in expected:
            assert hasattr(AgentRole, role_name)

    def test_role_values(self):
        """Verify role values are lowercase strings."""
        assert AgentRole.ARCHITECT.value == "architect"
        assert AgentRole.EXECUTOR.value == "executor"


class TestAgent:
    """Tests for Agent class."""

    def test_agent_creation(self, mock_runner):
        """Test basic agent creation."""
        agent = Agent(
            name="test-agent",
            runner=mock_runner,
            role=AgentRole.EXECUTOR
        )

        assert agent.name == "test-agent"
        assert agent.role == AgentRole.EXECUTOR
        assert agent.runner == mock_runner

    def test_agent_invoke(self, mock_runner):
        """Test agent invocation."""
        agent = Agent(name="test", runner=mock_runner)
        response = agent.invoke("Test prompt")

        assert response.success
        assert response.content == "Mock response for testing"

    def test_agent_invoke_with_context(self, echo_runner):
        """Test agent passes context correctly."""
        agent = Agent(name="test", runner=echo_runner)
        response = agent.invoke("Task here", context="Previous context")

        # Echo runner returns "Echo: {prompt}"
        assert "Previous context" in response.content
        assert "Task here" in response.content

    def test_agent_invoke_with_system_prompt(self, echo_runner):
        """Test agent includes system prompt."""
        agent = Agent(
            name="test",
            runner=echo_runner,
            system_prompt="You are a helpful assistant."
        )
        response = agent.invoke("Do something")

        assert "You are a helpful assistant" in response.content
        assert "Do something" in response.content

    def test_agent_with_runner_creates_copy(self, mock_runner):
        """Test with_runner creates a new agent."""
        agent1 = Agent(name="test", runner=mock_runner, role=AgentRole.ARCHITECT)
        new_runner = MockRunner(name="new-mock")
        agent2 = agent1.with_runner(new_runner)

        assert agent1 is not agent2
        assert agent1.runner is mock_runner
        assert agent2.runner is new_runner
        assert agent2.name == agent1.name
        assert agent2.role == agent1.role

    def test_agent_with_prompt_creates_copy(self, mock_runner):
        """Test with_prompt creates a new agent."""
        agent1 = Agent(name="test", runner=mock_runner, system_prompt="Original")
        agent2 = agent1.with_prompt("New prompt")

        assert agent1 is not agent2
        assert agent1.system_prompt == "Original"
        assert agent2.system_prompt == "New prompt"

    def test_agent_tracks_metadata(self, mock_runner):
        """Test response includes metadata."""
        agent = Agent(name="meta-test", runner=mock_runner, role=AgentRole.CRITIC)
        response = agent.invoke("Test")

        assert response.metadata["agent"] == "meta-test"
        assert response.metadata["role"] == "critic"
        assert response.metadata["backend"] == "mock"


class TestVerdictParsing:
    """Tests for verdict parsing in agent responses."""

    def test_verdict_pass_parsing(self):
        """Test VERDICT: PASS is parsed."""
        runner = MockRunner(responses=["Review complete.\n\nVERDICT: PASS"])
        agent = Agent(name="critic", runner=runner)
        response = agent.invoke("Review code")

        assert response.verdict == "PASS"
        assert response.passed is True

    def test_verdict_fail_parsing(self):
        """Test VERDICT: FAIL is parsed."""
        runner = MockRunner(responses=["Issues found.\n\nVERDICT: FAIL"])
        agent = Agent(name="critic", runner=runner)
        response = agent.invoke("Review code")

        assert response.verdict == "FAIL"
        assert response.failed is True

    def test_verdict_needs_revision(self):
        """Test NEEDS_REVISION is parsed."""
        runner = MockRunner(responses=["Minor issues.\n\nNEEDS_REVISION"])
        agent = Agent(name="critic", runner=runner)
        response = agent.invoke("Review code")

        assert response.verdict == "NEEDS_REVISION"

    def test_no_verdict_in_response(self):
        """Test responses without verdict."""
        runner = MockRunner(responses=["Just a regular response"])
        agent = Agent(name="test", runner=runner)
        response = agent.invoke("Test")

        assert response.verdict is None

    def test_verdict_case_insensitive(self):
        """Test verdict parsing is case insensitive."""
        runner = MockRunner(responses=["verdict: pass"])
        agent = Agent(name="test", runner=runner)
        response = agent.invoke("Test")

        assert response.verdict == "PASS"


class TestCreateAgentFactory:
    """Tests for create_agent factory function."""

    def test_create_agent_basic(self, mock_runner):
        """Test basic factory usage."""
        agent = create_agent("test", mock_runner)

        assert agent.name == "test"
        assert agent.role == AgentRole.CUSTOM

    def test_create_agent_with_role(self, mock_runner):
        """Test factory with role gets default prompt."""
        agent = create_agent("arch", mock_runner, AgentRole.ARCHITECT)

        assert agent.role == AgentRole.ARCHITECT
        assert agent.system_prompt == PROMPTS["architect"]

    def test_create_agent_custom_prompt_overrides(self, mock_runner):
        """Test custom prompt overrides default."""
        custom = "Custom prompt here"
        agent = create_agent(
            "test",
            mock_runner,
            AgentRole.ARCHITECT,
            system_prompt=custom
        )

        assert agent.system_prompt == custom


class TestAgentRepr:
    """Tests for agent string representation."""

    def test_repr_format(self, mock_runner):
        """Test __repr__ format."""
        agent = Agent(name="my-agent", runner=mock_runner, role=AgentRole.EXECUTOR)
        repr_str = repr(agent)

        assert "my-agent" in repr_str
        assert "executor" in repr_str
        assert "mock" in repr_str


@pytest.mark.unit
class TestAgentErrorHandling:
    """Tests for agent error handling."""

    def test_agent_handles_runner_failure(self, error_runner):
        """Test agent handles runner errors gracefully."""
        agent = Agent(name="test", runner=error_runner)
        response = agent.invoke("Test")

        assert response.success is False

    def test_agent_empty_prompt(self, mock_runner):
        """Test agent handles empty prompt."""
        agent = Agent(name="test", runner=mock_runner)
        response = agent.invoke("")

        # Should still work
        assert response is not None
