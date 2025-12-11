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


def _check_gemini():
    try:
        return GeminiRunner(backend="api").is_available()
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
            "gemini": _check_gemini(),
            "ollama-local": _check_ollama_local(),
            "ollama-home": _check_ollama_home(),
        }
    return _STATUS


def pytest_configure(config):
    config.addinivalue_line("markers", "requires_api: needs real backend")


def pytest_collection_modifyitems(config, items):
    status = get_status()
    skip = pytest.mark.skip(reason="No backends available")

    for item in items:
        if "requires_api" in item.keywords and not any(status.values()):
            item.add_marker(skip)


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
    if status["gemini"]:
        return GeminiRunner(backend="api")
    if status["ollama-local"]:
        return OllamaRunner(host=OLLAMA_LOCAL)
    if status["ollama-home"]:
        return OllamaRunner(host=OLLAMA_HOME)
    pytest.skip("No backends available")
