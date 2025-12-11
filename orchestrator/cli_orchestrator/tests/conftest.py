"""
Pytest configuration and fixtures for CLI Orchestrator tests.

Two test tiers:
1. Mock tests (fast, no API) - run with: pytest -m "not requires_api"
2. Real API tests (slower, requires backends) - run with: pytest -m requires_api

IMPORTANT: Real API tests are the source of truth. Mocks are for CI speed only.
"""
import sys
import pytest
import tempfile
import shutil
from pathlib import Path
from typing import Dict, Optional

# Add parent paths for imports
TEST_DIR = Path(__file__).parent
PACKAGE_DIR = TEST_DIR.parent
sys.path.insert(0, str(PACKAGE_DIR))
sys.path.insert(0, str(TEST_DIR))

from runners import ClaudeRunner, GeminiRunner, CLIRunner
from runners.custom_runner import OllamaRunner
from agent import Agent, AgentRole, create_agent
from memory_bank import MemoryBank
from mocks.mock_runners import (
    MockRunner,
    PassingCriticRunner,
    FailingCriticRunner,
    CountingRunner,
    ErrorRunner,
    EchoRunner,
)


# =============================================================================
# Backend Availability Detection
# =============================================================================

def _check_claude_available() -> bool:
    """Check if Claude CLI is installed and functional."""
    try:
        runner = ClaudeRunner()
        return runner.is_available()
    except Exception:
        return False


def _check_gemini_available() -> bool:
    """Check if Gemini API is available."""
    try:
        runner = GeminiRunner(backend="api")
        return runner.is_available()
    except Exception:
        return False


def _check_ollama_available() -> bool:
    """Check if Ollama is running."""
    try:
        runner = OllamaRunner()
        return runner.is_available()
    except Exception:
        return False


# Cache availability checks (expensive operations)
_BACKEND_STATUS: Dict[str, bool] = {}


def get_backend_status() -> Dict[str, bool]:
    """Get cached backend availability status."""
    global _BACKEND_STATUS
    if not _BACKEND_STATUS:
        _BACKEND_STATUS = {
            "claude": _check_claude_available(),
            "gemini": _check_gemini_available(),
            "ollama": _check_ollama_available(),
        }
    return _BACKEND_STATUS


# =============================================================================
# Pytest Hooks
# =============================================================================

def pytest_configure(config):
    """Register custom markers."""
    config.addinivalue_line("markers", "requires_api: Tests that require real API access")
    config.addinivalue_line("markers", "requires_claude: Tests that require Claude CLI")
    config.addinivalue_line("markers", "requires_gemini: Tests that require Gemini API")
    config.addinivalue_line("markers", "requires_ollama: Tests that require Ollama")


def pytest_collection_modifyitems(config, items):
    """Skip tests based on backend availability."""
    status = get_backend_status()

    skip_claude = pytest.mark.skip(reason="Claude CLI not available")
    skip_gemini = pytest.mark.skip(reason="Gemini API not available")
    skip_ollama = pytest.mark.skip(reason="Ollama not available")
    skip_api = pytest.mark.skip(reason="No API backends available")

    for item in items:
        if "requires_claude" in item.keywords and not status["claude"]:
            item.add_marker(skip_claude)
        if "requires_gemini" in item.keywords and not status["gemini"]:
            item.add_marker(skip_gemini)
        if "requires_ollama" in item.keywords and not status["ollama"]:
            item.add_marker(skip_ollama)
        if "requires_api" in item.keywords:
            if not any(status.values()):
                item.add_marker(skip_api)


def pytest_report_header(config):
    """Print backend status at test start."""
    status = get_backend_status()
    lines = ["Backend Availability:"]
    for backend, available in status.items():
        symbol = "✓" if available else "✗"
        lines.append(f"  {symbol} {backend}")
    return lines


# =============================================================================
# Mock Fixtures (for fast unit tests)
# =============================================================================

@pytest.fixture
def mock_runner():
    """Basic mock runner with preset response."""
    return MockRunner(responses=["Mock response for testing"])


@pytest.fixture
def passing_critic():
    """Mock critic that always passes."""
    return PassingCriticRunner()


@pytest.fixture
def failing_critic():
    """Mock critic that fails twice then passes."""
    return FailingCriticRunner(fail_count=2)


@pytest.fixture
def counting_runner():
    """Mock runner that counts invocations."""
    return CountingRunner()


@pytest.fixture
def error_runner():
    """Mock runner that always fails."""
    return ErrorRunner()


@pytest.fixture
def echo_runner():
    """Mock runner that echoes input."""
    return EchoRunner()


# =============================================================================
# Real Backend Fixtures (for integration tests)
# =============================================================================

@pytest.fixture
def claude_runner():
    """Real Claude CLI runner."""
    status = get_backend_status()
    if not status["claude"]:
        pytest.skip("Claude CLI not available")
    return ClaudeRunner(debug=True)


@pytest.fixture
def gemini_runner():
    """Real Gemini API runner."""
    status = get_backend_status()
    if not status["gemini"]:
        pytest.skip("Gemini API not available")
    return GeminiRunner(backend="api")


@pytest.fixture
def ollama_runner():
    """Real Ollama runner."""
    status = get_backend_status()
    if not status["ollama"]:
        pytest.skip("Ollama not available")
    return OllamaRunner(model="llama3.1:8b")


@pytest.fixture
def any_real_runner():
    """Any available real runner (for generic tests)."""
    status = get_backend_status()
    if status["claude"]:
        return ClaudeRunner()
    if status["gemini"]:
        return GeminiRunner(backend="api")
    if status["ollama"]:
        return OllamaRunner()
    pytest.skip("No backends available")


# =============================================================================
# Agent Fixtures
# =============================================================================

@pytest.fixture
def mock_architect(mock_runner):
    """Mock architect agent."""
    return create_agent("test-architect", mock_runner, AgentRole.ARCHITECT)


@pytest.fixture
def mock_executor(mock_runner):
    """Mock executor agent."""
    return create_agent("test-executor", mock_runner, AgentRole.EXECUTOR)


@pytest.fixture
def real_architect(any_real_runner):
    """Real architect agent with any available backend."""
    return create_agent("real-architect", any_real_runner, AgentRole.ARCHITECT)


@pytest.fixture
def real_executor(any_real_runner):
    """Real executor agent with any available backend."""
    return create_agent("real-executor", any_real_runner, AgentRole.EXECUTOR)


# =============================================================================
# Memory Bank Fixtures
# =============================================================================

@pytest.fixture
def temp_memory_bank(tmp_path):
    """Memory bank in a temporary directory."""
    mb = MemoryBank(base_dir=tmp_path / ".orchestrator")
    yield mb
    # Cleanup happens automatically with tmp_path


@pytest.fixture
def initialized_memory_bank(temp_memory_bank):
    """Memory bank initialized with a goal."""
    temp_memory_bank.initialize(
        goal="Test goal for testing",
        project_info="Test project"
    )
    return temp_memory_bank


# =============================================================================
# Test Data Fixtures
# =============================================================================

@pytest.fixture
def simple_prompt():
    """Simple test prompt."""
    return "What is 2 + 2?"


@pytest.fixture
def code_task():
    """Code generation task."""
    return "Write a Python function that calculates factorial"


@pytest.fixture
def review_task():
    """Code review task."""
    return """Review this code:
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)
"""


@pytest.fixture
def complex_task():
    """Complex multi-step task."""
    return """Design and implement a REST API endpoint for user registration.
Requirements:
- POST /api/users/register
- Validate email format
- Hash password before storing
- Return user ID on success
"""
