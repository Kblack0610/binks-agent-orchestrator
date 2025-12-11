"""
REAL Backend Integration Tests

These tests ACTUALLY call the backends. No mocks. No bullshit.
If these pass, the system works. If they fail, it's broken.

Run with: pytest -m requires_api -v
"""
import sys
import pytest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from agent import Agent, AgentRole, create_agent


# =============================================================================
# Core Tests - These MUST pass for the system to work
# =============================================================================

@pytest.mark.requires_api
class TestBasicFunctionality:
    """Tests that verify the core system actually works."""

    def test_runner_responds(self, any_real_runner):
        """Can we get ANY response from ANY backend?"""
        result = any_real_runner.run("Say 'hello'")

        assert result.success, f"Runner failed: {result.error}"
        assert result.content, "Empty response"
        assert len(result.content) > 0

    def test_agent_invoke_works(self, any_real_runner):
        """Can an Agent invoke a runner and get a response?"""
        agent = create_agent("test", any_real_runner, AgentRole.EXECUTOR)

        response = agent.invoke("What is 2+2? Reply with just the number.")

        assert response.success
        assert response.content
        assert "4" in response.content

    def test_verdict_parsing_works(self, any_real_runner):
        """Does verdict parsing ACTUALLY work with real AI responses?

        This test MUST fail if the AI doesn't return a parseable verdict.
        """
        agent = create_agent(
            name="critic",
            runner=any_real_runner,
            role=AgentRole.CRITIC
        )

        response = agent.invoke(
            "Is 2+2=4 correct? Reply with VERDICT: PASS or VERDICT: FAIL"
        )

        assert response.success
        assert response.verdict in ["PASS", "FAIL"], \
            f"Verdict not parsed! Response ended with: ...{response.content[-100:]}"


@pytest.mark.requires_api
class TestMoAWorkflow:
    """Tests that verify the MoA workflow actually works end-to-end."""

    def test_orchestrator_runs_without_crashing(self, any_real_runner):
        """Does run_moa_workflow execute without errors?"""
        from orchestrator import Orchestrator, ConvergenceCriteria

        architect = create_agent("arch", any_real_runner, AgentRole.ARCHITECT)
        executor = create_agent("exec", any_real_runner, AgentRole.EXECUTOR)

        orch = Orchestrator()
        convergence = ConvergenceCriteria(max_iterations=1)

        conversation = orch.run_moa_workflow(
            goal="Write: print('hi')",
            architect=architect,
            executor=executor,
            convergence=convergence
        )

        assert conversation is not None
        assert len(conversation.turns) >= 2  # At least plan + implement

    def test_critic_produces_verdict(self, any_real_runner):
        """Does the orchestrator's critic prompt produce a parseable VERDICT?

        This is the test that would have caught the VERDICT bug.
        """
        from orchestrator import Orchestrator, ConvergenceCriteria

        architect = create_agent("arch", any_real_runner, AgentRole.ARCHITECT)
        executor = create_agent("exec", any_real_runner, AgentRole.EXECUTOR)
        critic = create_agent("critic", any_real_runner, AgentRole.CRITIC)

        orch = Orchestrator()
        convergence = ConvergenceCriteria(max_iterations=1)

        conversation = orch.run_moa_workflow(
            goal="Write: print('hi')",
            architect=architect,
            executor=executor,
            critic=critic,
            convergence=convergence
        )

        # Should have at least 3 turns: plan, implement, review
        assert len(conversation.turns) >= 3, f"Only {len(conversation.turns)} turns"

        # Third turn should be the review - check it has verdict
        review_response = conversation.turns[2].response.upper()
        has_verdict = "VERDICT: PASS" in review_response or "VERDICT: FAIL" in review_response

        assert has_verdict, \
            f"Critic didn't return VERDICT! Got: ...{conversation.turns[2].response[-150:]}"
