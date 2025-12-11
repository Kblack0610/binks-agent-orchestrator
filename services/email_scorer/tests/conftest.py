"""
Pytest configuration for Email Scorer tests.

Reuses markers from the main orchestrator test suite.
"""
import sys
import pytest
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent.parent.parent
ORCHESTRATOR_PATH = PROJECT_ROOT / "orchestrator" / "cli_orchestrator"
sys.path.insert(0, str(ORCHESTRATOR_PATH))

from runners import ClaudeRunner, GeminiRunner


def _check_claude():
    try:
        return ClaudeRunner().is_available()
    except:
        return False


def _check_gemini_cli():
    try:
        return GeminiRunner(backend="gemini").is_available()
    except:
        return False


def _check_gemini_api():
    try:
        return GeminiRunner(backend="api").is_available()
    except:
        return False


_STATUS = {}


def get_status():
    global _STATUS
    if not _STATUS:
        _STATUS = {
            "claude": _check_claude(),
            "gemini-cli": _check_gemini_cli(),
            "gemini-api": _check_gemini_api(),
        }
    return _STATUS


def pytest_configure(config):
    config.addinivalue_line("markers", "requires_api: needs real backend")
    config.addinivalue_line("markers", "requires_gemini_cli: needs Gemini CLI")


def pytest_collection_modifyitems(config, items):
    status = get_status()

    for item in items:
        if "requires_api" in item.keywords and not any(status.values()):
            item.add_marker(pytest.mark.skip(reason="No backends available"))

        if "requires_gemini_cli" in item.keywords and not status["gemini-cli"]:
            item.add_marker(pytest.mark.skip(reason="Gemini CLI not available"))


def pytest_report_header(config):
    status = get_status()
    lines = ["Email Scorer - Available Backends:"]
    for name, ok in status.items():
        lines.append(f"  {'✓' if ok else '✗'} {name}")
    return lines
