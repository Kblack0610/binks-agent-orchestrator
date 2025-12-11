"""
Mock runners for testing the CLI Orchestrator.

These mocks allow testing without actual API calls or CLI installations.
"""
import sys
from pathlib import Path
from typing import Optional, List, Dict, Any, Callable

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from runners.base import CLIRunner, RunnerResult


class MockRunner(CLIRunner):
    """
    Configurable mock runner for testing.

    Can be configured with:
    - Preset responses (list or single)
    - Response function (dynamic responses)
    - Failure simulation
    - Latency simulation
    """

    def __init__(
        self,
        name: str = "mock",
        responses: Optional[List[str]] = None,
        response_fn: Optional[Callable[[str], str]] = None,
        should_fail: bool = False,
        error_message: str = "Mock error",
        latency: float = 0.0,
        model: str = "mock-model",
        **kwargs
    ):
        super().__init__(
            name=name,
            executable="mock",
            **kwargs
        )
        self.responses = responses or ["Mock response"]
        self.response_fn = response_fn
        self.should_fail = should_fail
        self.error_message = error_message
        self.latency = latency
        self.model = model
        self._call_count = 0
        self._call_history: List[str] = []

    def run(self, prompt: str, **kwargs) -> RunnerResult:
        """Execute a mock prompt and return result."""
        self._call_count += 1
        self._call_history.append(prompt)

        if self.should_fail:
            return RunnerResult(
                content="",
                success=False,
                error=self.error_message,
                backend=self.name,
                model=self.model,
                execution_time=self.latency
            )

        # Get response
        if self.response_fn:
            content = self.response_fn(prompt)
        else:
            # Cycle through responses
            idx = (self._call_count - 1) % len(self.responses)
            content = self.responses[idx]

        return RunnerResult(
            content=content,
            success=True,
            backend=self.name,
            model=self.model,
            execution_time=self.latency,
            session_id=f"mock-session-{self._call_count}"
        )

    def is_available(self) -> bool:
        """Mock is always available."""
        return True

    @property
    def call_count(self) -> int:
        """Number of times run() was called."""
        return self._call_count

    @property
    def call_history(self) -> List[str]:
        """List of prompts passed to run()."""
        return self._call_history

    def reset(self) -> None:
        """Reset call tracking."""
        self._call_count = 0
        self._call_history = []


class PassingCriticRunner(MockRunner):
    """
    Mock runner that always returns VERDICT: PASS.

    Useful for testing successful workflow completion.
    """

    def __init__(self, name: str = "passing-critic", **kwargs):
        super().__init__(
            name=name,
            responses=["Code review complete. All checks passed.\n\nVERDICT: PASS"],
            **kwargs
        )


class FailingCriticRunner(MockRunner):
    """
    Mock runner that fails N times then passes.

    Useful for testing convergence loops.

    Args:
        fail_count: Number of times to return FAIL before PASS
    """

    def __init__(
        self,
        name: str = "failing-critic",
        fail_count: int = 2,
        **kwargs
    ):
        super().__init__(name=name, **kwargs)
        self.fail_count = fail_count
        self._fail_counter = 0

    def run(self, prompt: str, **kwargs) -> RunnerResult:
        """Return FAIL for first N calls, then PASS."""
        self._call_count += 1
        self._call_history.append(prompt)
        self._fail_counter += 1

        if self._fail_counter <= self.fail_count:
            content = f"Issues found (attempt {self._fail_counter}):\n- Bug in line 42\n- Missing error handling\n\nVERDICT: FAIL"
        else:
            content = "All issues resolved. Code looks good.\n\nVERDICT: PASS"

        return RunnerResult(
            content=content,
            success=True,
            backend=self.name,
            model=self.model,
            execution_time=self.latency
        )

    def reset(self) -> None:
        """Reset counters."""
        super().reset()
        self._fail_counter = 0


class CountingRunner(MockRunner):
    """
    Mock runner that includes call count in responses.

    Useful for verifying number of invocations.
    """

    def __init__(self, name: str = "counting", **kwargs):
        super().__init__(name=name, **kwargs)

    def run(self, prompt: str, **kwargs) -> RunnerResult:
        """Return response with call count."""
        self._call_count += 1
        self._call_history.append(prompt)

        return RunnerResult(
            content=f"Response #{self._call_count}",
            success=True,
            backend=self.name,
            model=self.model,
            execution_time=self.latency
        )


class ErrorRunner(MockRunner):
    """
    Mock runner that always fails.

    Useful for testing error handling paths.
    """

    def __init__(
        self,
        name: str = "error",
        error_message: str = "Simulated error",
        **kwargs
    ):
        super().__init__(
            name=name,
            should_fail=True,
            error_message=error_message,
            **kwargs
        )


class SequenceRunner(MockRunner):
    """
    Mock runner with a specific sequence of responses.

    Useful for testing multi-step workflows with predictable outputs.
    """

    def __init__(
        self,
        name: str = "sequence",
        sequence: Optional[List[str]] = None,
        **kwargs
    ):
        super().__init__(
            name=name,
            responses=sequence or [
                "Step 1: Planning complete",
                "Step 2: Implementation done",
                "Step 3: Review passed\n\nVERDICT: PASS"
            ],
            **kwargs
        )


class EchoRunner(MockRunner):
    """
    Mock runner that echoes back the prompt.

    Useful for testing prompt construction.
    """

    def __init__(self, name: str = "echo", prefix: str = "Echo: ", **kwargs):
        super().__init__(name=name, **kwargs)
        self.prefix = prefix

    def run(self, prompt: str, **kwargs) -> RunnerResult:
        """Echo the prompt back."""
        self._call_count += 1
        self._call_history.append(prompt)

        return RunnerResult(
            content=f"{self.prefix}{prompt}",
            success=True,
            backend=self.name,
            model=self.model,
            execution_time=self.latency
        )
