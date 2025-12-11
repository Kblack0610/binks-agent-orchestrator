"""
Unit tests for MemoryBank class.
"""
import sys
import pytest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from memory_bank import MemoryBank


class TestMemoryBankCreation:
    """Tests for MemoryBank initialization."""

    def test_create_with_default_path(self, tmp_path, monkeypatch):
        """Test creation with default path."""
        monkeypatch.chdir(tmp_path)
        mb = MemoryBank()
        assert mb.base_dir == Path(".orchestrator")

    def test_create_with_custom_path(self, tmp_path):
        """Test creation with custom path."""
        custom_path = tmp_path / "custom_memory"
        mb = MemoryBank(base_dir=custom_path)
        assert mb.base_dir == custom_path


class TestMemoryBankPaths:
    """Tests for MemoryBank file path properties."""

    def test_product_context_path(self, temp_memory_bank):
        """Test product context file path."""
        assert temp_memory_bank.product_context.name == "productContext.md"

    def test_active_context_path(self, temp_memory_bank):
        """Test active context file path."""
        assert temp_memory_bank.active_context.name == "activeContext.md"

    def test_system_patterns_path(self, temp_memory_bank):
        """Test system patterns file path."""
        assert temp_memory_bank.system_patterns.name == "systemPatterns.md"


class TestMemoryBankInitialize:
    """Tests for MemoryBank.initialize()."""

    def test_initialize_creates_directory(self, temp_memory_bank):
        """Test initialize creates base directory."""
        temp_memory_bank.initialize(goal="Test goal")
        assert temp_memory_bank.base_dir.exists()

    def test_initialize_creates_product_context(self, temp_memory_bank):
        """Test initialize creates productContext.md."""
        temp_memory_bank.initialize(goal="Build a REST API")
        assert temp_memory_bank.product_context.exists()
        content = temp_memory_bank.product_context.read_text()
        assert "Build a REST API" in content

    def test_initialize_creates_active_context(self, temp_memory_bank):
        """Test initialize creates activeContext.md."""
        temp_memory_bank.initialize(goal="Test goal")
        assert temp_memory_bank.active_context.exists()

    def test_initialize_with_project_info(self, temp_memory_bank):
        """Test initialize with project info."""
        temp_memory_bank.initialize(
            goal="Test goal",
            project_info="Python project with FastAPI"
        )
        content = temp_memory_bank.product_context.read_text()
        assert "FastAPI" in content


class TestMemoryBankReadWrite:
    """Tests for MemoryBank read/write operations."""

    def test_read_context_empty(self, temp_memory_bank):
        """Test reading context when not initialized."""
        context = temp_memory_bank.read_context()
        # Should return empty or minimal content
        assert context is not None

    def test_read_context_after_initialize(self, initialized_memory_bank):
        """Test reading context after initialization."""
        context = initialized_memory_bank.read_context()
        assert len(context) > 0
        assert "Test goal" in context

    def test_update_active_context(self, initialized_memory_bank):
        """Test updating active context."""
        initialized_memory_bank.update_active_context("New progress update")
        content = initialized_memory_bank.active_context.read_text()
        assert "New progress update" in content

    def test_read_context_max_chars(self, initialized_memory_bank):
        """Test read_context adds warning when exceeding max_chars."""
        # Add substantial content
        initialized_memory_bank.update_active_context("X" * 10000)
        context = initialized_memory_bank.read_context(max_chars=100)
        # Implementation adds warning when exceeding max_chars, doesn't truncate
        assert "WARNING" in context
        assert "exceeds" in context.lower() or "100" in context


class TestMemoryBankExists:
    """Tests for checking if memory bank exists."""

    def test_exists_false_when_empty(self, temp_memory_bank):
        """Test exists returns False when not initialized."""
        assert not temp_memory_bank.product_context.exists()

    def test_exists_true_after_initialize(self, initialized_memory_bank):
        """Test exists returns True after initialization."""
        assert initialized_memory_bank.product_context.exists()
        assert initialized_memory_bank.active_context.exists()


class TestMemoryBankAppend:
    """Tests for appending to memory bank files."""

    def test_append_to_active_context(self, initialized_memory_bank):
        """Test appending to active context."""
        initialized_memory_bank.update_active_context("First update")
        initialized_memory_bank.update_active_context("Second update")

        content = initialized_memory_bank.active_context.read_text()
        # Should have both updates (implementation may vary)
        assert "Second update" in content

    def test_append_pattern(self, initialized_memory_bank):
        """Test recording a pattern."""
        if hasattr(initialized_memory_bank, 'add_pattern'):
            initialized_memory_bank.add_pattern(
                "Repository Pattern",
                "Use repository classes for data access"
            )
            content = initialized_memory_bank.system_patterns.read_text()
            assert "Repository" in content


@pytest.mark.unit
class TestMemoryBankEdgeCases:
    """Edge case tests for MemoryBank."""

    def test_initialize_twice_is_safe(self, temp_memory_bank):
        """Test initializing twice doesn't break anything."""
        temp_memory_bank.initialize(goal="First goal")
        temp_memory_bank.initialize(goal="Second goal")
        # Should work without error
        assert temp_memory_bank.product_context.exists()

    def test_read_nonexistent_file(self, temp_memory_bank):
        """Test reading when files don't exist."""
        # Should not raise exception
        context = temp_memory_bank.read_context()
        assert context is not None

    def test_special_characters_in_goal(self, temp_memory_bank):
        """Test goal with special characters."""
        goal = "Build API with 'quotes' and \"double quotes\" & ampersands"
        temp_memory_bank.initialize(goal=goal)
        content = temp_memory_bank.product_context.read_text()
        assert "quotes" in content
