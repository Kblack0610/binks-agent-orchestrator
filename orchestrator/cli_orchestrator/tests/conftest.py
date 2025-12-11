"""
Pytest fixtures for REAL backend tests.

No mocks. Tests use actual Claude/Gemini/Ollama.
"""
import sys
import pytest
from pathlib import Path

TEST_DIR = Path(__file__).parent
PACKAGE_DIR = TEST_DIR.parent
sys.path.insert(0, str(PACKAGE_DIR))

from runners import ClaudeRunner, GeminiRunner
from runners.custom_runner import OllamaRunner, OLLAMA_LOCAL, OLLAMA_HOME


def _check_claude():
    try:
        return ClaudeRunner().is_available()
    except:
        return False


def _check_gemini_api():
    try:
        return GeminiRunner(backend="api").is_available()
    except:
        return False


def _check_gemini_cli():
    try:
        return GeminiRunner(backend="gemini").is_available()
    except:
        return False


def _check_ollama_local():
    try:
        return OllamaRunner(host=OLLAMA_LOCAL).is_available()
    except:
        return False


def _check_ollama_home():
    try:
        return OllamaRunner(host=OLLAMA_HOME).is_available()
    except:
        return False


# Cache status
_STATUS = {}


def get_status():
    global _STATUS
    if not _STATUS:
        _STATUS = {
            "claude": _check_claude(),
            "gemini-api": _check_gemini_api(),
            "gemini-cli": _check_gemini_cli(),
            "ollama-local": _check_ollama_local(),
            "ollama-home": _check_ollama_home(),
        }
    return _STATUS


def pytest_configure(config):
    config.addinivalue_line("markers", "requires_api: needs real backend")
    config.addinivalue_line("markers", "requires_claude: needs Claude backend")
    config.addinivalue_line("markers", "requires_gemini: needs Gemini backend (API or CLI)")
    config.addinivalue_line("markers", "requires_gemini_cli: needs Gemini CLI specifically")
    config.addinivalue_line("markers", "requires_ollama: needs Ollama backend")


def pytest_collection_modifyitems(config, items):
    status = get_status()

    for item in items:
        # Skip if no backends available at all
        if "requires_api" in item.keywords and not any(status.values()):
            item.add_marker(pytest.mark.skip(reason="No backends available"))

        # Skip Claude-specific tests
        if "requires_claude" in item.keywords and not status["claude"]:
            item.add_marker(pytest.mark.skip(reason="Claude not available"))

        # Skip Gemini-specific tests (either API or CLI)
        if "requires_gemini" in item.keywords:
            if not status["gemini-api"] and not status["gemini-cli"]:
                item.add_marker(pytest.mark.skip(reason="Gemini not available"))

        # Skip Gemini CLI-specific tests
        if "requires_gemini_cli" in item.keywords and not status["gemini-cli"]:
            item.add_marker(pytest.mark.skip(reason="Gemini CLI not available"))

        # Skip Ollama-specific tests
        if "requires_ollama" in item.keywords:
            if not status["ollama-local"] and not status["ollama-home"]:
                item.add_marker(pytest.mark.skip(reason="Ollama not available"))


def pytest_report_header(config):
    status = get_status()
    lines = ["Backends:"]
    for name, ok in status.items():
        lines.append(f"  {'✓' if ok else '✗'} {name}")
    return lines


@pytest.fixture
def any_real_runner():
    """Returns first available real runner."""
    status = get_status()
    if status["claude"]:
        return ClaudeRunner()
    if status["gemini-cli"]:
        return GeminiRunner(backend="gemini")
    if status["gemini-api"]:
        return GeminiRunner(backend="api")
    if status["ollama-local"]:
        return OllamaRunner(host=OLLAMA_LOCAL)
    if status["ollama-home"]:
        return OllamaRunner(host=OLLAMA_HOME)
    pytest.skip("No backends available")


@pytest.fixture
def claude_runner():
    """Returns Claude runner if available."""
    if not _check_claude():
        pytest.skip("Claude not available")
    return ClaudeRunner()


@pytest.fixture
def gemini_cli_runner():
    """Returns Gemini CLI runner if available."""
    if not _check_gemini_cli():
        pytest.skip("Gemini CLI not available")
    return GeminiRunner(backend="gemini")


@pytest.fixture
def gemini_api_runner():
    """Returns Gemini API runner if available."""
    if not _check_gemini_api():
        pytest.skip("Gemini API not available")
    return GeminiRunner(backend="api")


@pytest.fixture
def ollama_runner():
    """Returns first available Ollama runner."""
    status = get_status()
    if status["ollama-local"]:
        return OllamaRunner(host=OLLAMA_LOCAL)
    if status["ollama-home"]:
        return OllamaRunner(host=OLLAMA_HOME)
    pytest.skip("Ollama not available")
