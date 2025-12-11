"""
Performance benchmark tests.

These tests ensure operations stay within acceptable time limits.
Run with: pytest -m benchmark
"""
import sys
import time
import pytest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from agent import Agent, AgentRole, create_agent
from memory_bank import MemoryBank
from tests.mocks.mock_runners import MockRunner


@pytest.mark.benchmark
class TestAgentPerformance:
    """Benchmark tests for Agent class."""

    def test_agent_invoke_speed(self, mock_runner):
        """Agent.invoke() should be fast without API call."""
        agent = create_agent("bench", mock_runner, AgentRole.EXECUTOR)

        start = time.perf_counter()
        for _ in range(100):
            agent.invoke("Test prompt")
        elapsed = time.perf_counter() - start

        avg_ms = (elapsed / 100) * 1000
        assert avg_ms < 1, f"Agent.invoke() too slow: {avg_ms:.2f}ms average"

    def test_agent_creation_speed(self, mock_runner):
        """Agent creation should be instantaneous."""
        start = time.perf_counter()
        for _ in range(1000):
            Agent(name="bench", runner=mock_runner)
        elapsed = time.perf_counter() - start

        avg_us = (elapsed / 1000) * 1_000_000
        assert avg_us < 100, f"Agent creation too slow: {avg_us:.2f}μs average"

    def test_with_runner_speed(self, mock_runner):
        """with_runner() should be fast."""
        agent = create_agent("bench", mock_runner)
        new_runner = MockRunner(name="new")

        start = time.perf_counter()
        for _ in range(1000):
            agent.with_runner(new_runner)
        elapsed = time.perf_counter() - start

        avg_us = (elapsed / 1000) * 1_000_000
        assert avg_us < 100, f"with_runner() too slow: {avg_us:.2f}μs average"


@pytest.mark.benchmark
class TestMemoryBankPerformance:
    """Benchmark tests for MemoryBank."""

    def test_memory_bank_write_speed(self, tmp_path):
        """MemoryBank writes should be fast."""
        mb = MemoryBank(base_dir=tmp_path / ".orchestrator")
        mb.initialize(goal="Benchmark test")

        start = time.perf_counter()
        for i in range(100):
            mb.update_active_context(f"Update {i}: " + "x" * 100)
        elapsed = time.perf_counter() - start

        avg_ms = (elapsed / 100) * 1000
        assert avg_ms < 5, f"MemoryBank write too slow: {avg_ms:.2f}ms average"

    def test_memory_bank_read_speed(self, tmp_path):
        """MemoryBank reads should be fast."""
        mb = MemoryBank(base_dir=tmp_path / ".orchestrator")
        mb.initialize(goal="Benchmark test")
        # Add some content
        mb.update_active_context("x" * 10000)

        start = time.perf_counter()
        for _ in range(100):
            mb.read_context()
        elapsed = time.perf_counter() - start

        avg_ms = (elapsed / 100) * 1000
        assert avg_ms < 5, f"MemoryBank read too slow: {avg_ms:.2f}ms average"

    def test_memory_bank_initialize_speed(self, tmp_path):
        """MemoryBank initialization should be fast."""
        start = time.perf_counter()
        for i in range(50):
            mb = MemoryBank(base_dir=tmp_path / f".orchestrator_{i}")
            mb.initialize(goal=f"Test {i}")
        elapsed = time.perf_counter() - start

        avg_ms = (elapsed / 50) * 1000
        assert avg_ms < 10, f"MemoryBank init too slow: {avg_ms:.2f}ms average"


@pytest.mark.benchmark
class TestMockRunnerPerformance:
    """Benchmark tests for mock runners (baseline)."""

    def test_mock_runner_speed(self):
        """Mock runner should be essentially instantaneous."""
        runner = MockRunner()

        start = time.perf_counter()
        for _ in range(10000):
            runner.run("Test")
        elapsed = time.perf_counter() - start

        avg_us = (elapsed / 10000) * 1_000_000
        assert avg_us < 50, f"MockRunner too slow: {avg_us:.2f}μs average"


@pytest.mark.benchmark
class TestVerdictParsingPerformance:
    """Benchmark verdict parsing in responses."""

    def test_verdict_parsing_speed(self, mock_runner):
        """Verdict parsing should not add significant overhead."""
        # Response with verdict to parse
        runner = MockRunner(responses=["Long response " * 100 + "\n\nVERDICT: PASS"])
        agent = create_agent("bench", runner, AgentRole.CRITIC)

        start = time.perf_counter()
        for _ in range(1000):
            response = agent.invoke("Review")
            _ = response.verdict
        elapsed = time.perf_counter() - start

        avg_us = (elapsed / 1000) * 1_000_000
        assert avg_us < 500, f"Verdict parsing too slow: {avg_us:.2f}μs average"


@pytest.mark.benchmark
@pytest.mark.slow
class TestScalabilityBenchmarks:
    """Tests for scalability with larger inputs."""

    def test_large_prompt_handling(self, mock_runner):
        """Agent should handle large prompts efficiently."""
        agent = create_agent("bench", mock_runner)
        large_prompt = "x" * 100000  # 100KB prompt

        start = time.perf_counter()
        for _ in range(10):
            agent.invoke(large_prompt)
        elapsed = time.perf_counter() - start

        avg_ms = (elapsed / 10) * 1000
        assert avg_ms < 10, f"Large prompt handling too slow: {avg_ms:.2f}ms"

    def test_large_context_handling(self, mock_runner):
        """Agent should handle large context efficiently."""
        agent = create_agent("bench", mock_runner)
        large_context = "Previous output: " + "y" * 100000

        start = time.perf_counter()
        for _ in range(10):
            agent.invoke("Continue", context=large_context)
        elapsed = time.perf_counter() - start

        avg_ms = (elapsed / 10) * 1000
        assert avg_ms < 10, f"Large context handling too slow: {avg_ms:.2f}ms"
