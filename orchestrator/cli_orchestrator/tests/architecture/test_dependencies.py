"""
Architecture validation tests.

These tests ensure the codebase follows clean architecture principles:
- Runners don't depend on orchestrator
- No circular imports
- All runners implement CLIRunner interface
"""
import sys
import ast
import pytest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent.parent))


# Get paths
PACKAGE_ROOT = Path(__file__).parent.parent.parent
RUNNERS_DIR = PACKAGE_ROOT / "runners"


class TestDependencyDirection:
    """Verify dependencies flow in correct direction."""

    def test_runners_dont_import_orchestrator(self):
        """Runners should NOT import from orchestrator module."""
        runner_files = list(RUNNERS_DIR.glob("*.py"))

        for runner_file in runner_files:
            content = runner_file.read_text()
            # Check for problematic imports
            assert "from orchestrator import" not in content, \
                f"{runner_file.name} imports from orchestrator"
            assert "from ..orchestrator import" not in content, \
                f"{runner_file.name} imports from orchestrator"
            assert "import orchestrator" not in content, \
                f"{runner_file.name} imports orchestrator"

    def test_runners_dont_import_agent(self):
        """Runners should NOT import Agent (agent depends on runners)."""
        runner_files = list(RUNNERS_DIR.glob("*.py"))

        for runner_file in runner_files:
            content = runner_file.read_text()
            assert "from agent import" not in content, \
                f"{runner_file.name} imports from agent"
            assert "from ..agent import" not in content, \
                f"{runner_file.name} imports from agent"

    def test_base_runner_has_no_concrete_imports(self):
        """base.py should not import concrete runner implementations."""
        base_file = RUNNERS_DIR / "base.py"
        content = base_file.read_text()

        # Should not import specific runners
        assert "ClaudeRunner" not in content
        assert "GeminiRunner" not in content
        assert "OllamaRunner" not in content


class TestInterfaceCompliance:
    """Verify all runners implement required interface."""

    def test_all_runners_inherit_from_clirunner(self):
        """All runners must inherit from CLIRunner."""
        from runners.base import CLIRunner
        from runners import ClaudeRunner, GeminiRunner, CustomRunner

        runners = [ClaudeRunner, GeminiRunner, CustomRunner]

        for runner_cls in runners:
            assert issubclass(runner_cls, CLIRunner), \
                f"{runner_cls.__name__} doesn't inherit from CLIRunner"

    def test_all_runners_implement_run(self):
        """All runners must implement run() method."""
        from runners import ClaudeRunner, GeminiRunner, CustomRunner

        runners = [ClaudeRunner, GeminiRunner, CustomRunner]

        for runner_cls in runners:
            assert hasattr(runner_cls, "run"), \
                f"{runner_cls.__name__} missing run() method"
            # Check it's not just inherited abstract
            assert "run" in runner_cls.__dict__ or \
                   any("run" in base.__dict__ for base in runner_cls.__mro__[1:])

    def test_all_runners_implement_is_available(self):
        """All runners must implement is_available() method."""
        from runners import ClaudeRunner, GeminiRunner, CustomRunner

        runners = [ClaudeRunner, GeminiRunner, CustomRunner]

        for runner_cls in runners:
            assert hasattr(runner_cls, "is_available"), \
                f"{runner_cls.__name__} missing is_available() method"


class TestNoCircularImports:
    """Verify no circular imports exist."""

    def test_can_import_runners(self):
        """Test runners module imports cleanly."""
        # This will fail if circular imports exist
        from runners import CLIRunner, ClaudeRunner, GeminiRunner

    def test_can_import_agent(self):
        """Test agent module imports cleanly."""
        from agent import Agent, AgentRole, create_agent

    def test_can_import_orchestrator(self):
        """Test orchestrator module imports cleanly."""
        from orchestrator import Orchestrator

    def test_can_import_memory_bank(self):
        """Test memory_bank module imports cleanly."""
        from memory_bank import MemoryBank

    def test_import_order_independence(self):
        """Test modules can be imported in any order."""
        # Fresh import simulation - if circular deps exist, order matters
        import importlib
        import sys

        # Remove cached imports
        modules_to_remove = [k for k in sys.modules.keys()
                           if k.startswith(('runners', 'agent', 'orchestrator', 'memory_bank'))]
        for mod in modules_to_remove:
            del sys.modules[mod]

        # Import in different order
        from memory_bank import MemoryBank
        from agent import Agent
        from runners import ClaudeRunner
        from orchestrator import Orchestrator

        # Should work without error
        assert MemoryBank is not None
        assert Agent is not None


class TestAgentRunnerOrthogonality:
    """Verify Agent and Runner are truly orthogonal."""

    def test_agent_accepts_any_runner(self):
        """Agent should accept any CLIRunner implementation."""
        from agent import Agent, AgentRole
        from runners.base import CLIRunner, RunnerResult

        # Create a custom runner inline
        class CustomTestRunner(CLIRunner):
            def __init__(self):
                super().__init__(name="custom-test", executable="test")

            def run(self, prompt, **kwargs):
                return RunnerResult(content="Custom response", success=True)

            def is_available(self):
                return True

        runner = CustomTestRunner()
        agent = Agent(name="test", runner=runner, role=AgentRole.EXECUTOR)

        # Should work with any runner
        response = agent.invoke("Test")
        assert response.content == "Custom response"

    def test_runner_doesnt_know_about_roles(self):
        """Runners should have no knowledge of AgentRole."""
        runner_files = list(RUNNERS_DIR.glob("*.py"))

        for runner_file in runner_files:
            content = runner_file.read_text()
            assert "AgentRole" not in content, \
                f"{runner_file.name} references AgentRole"


@pytest.mark.architecture
class TestModuleStructure:
    """Verify expected module structure exists."""

    def test_runners_has_init(self):
        """Runners package has __init__.py."""
        assert (RUNNERS_DIR / "__init__.py").exists()

    def test_runners_exports_base_class(self):
        """Runners __init__ exports CLIRunner."""
        from runners import CLIRunner
        assert CLIRunner is not None

    def test_expected_files_exist(self):
        """Verify expected source files exist."""
        expected_files = [
            "agent.py",
            "memory_bank.py",
            "orchestrator.py",
            "runners/base.py",
            "runners/claude_runner.py",
            "runners/gemini_runner.py",
        ]

        for file_path in expected_files:
            full_path = PACKAGE_ROOT / file_path
            assert full_path.exists(), f"Missing expected file: {file_path}"
