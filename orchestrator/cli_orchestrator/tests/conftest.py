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
from runners.custom_runner import OllamaRunner


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


def _check_ollama():
    try:
        return OllamaRunner().is_available()
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
            "ollama": _check_ollama(),
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
    if status["ollama"]:
        return OllamaRunner()
    pytest.skip("No backends available")
