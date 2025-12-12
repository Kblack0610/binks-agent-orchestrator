"""
Tests for Feature Flag Configuration System.

Tests the centralized config management including:
- Feature flag toggling
- Config value management
- File persistence
- Environment variable loading
"""
import os
import sys
import json
import pytest
import tempfile
from pathlib import Path
from unittest.mock import patch

# Add parent to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from config import (
    ConfigManager,
    FeatureFlag,
    DEFAULT_FLAGS,
    DEFAULT_CONFIG,
    get_config,
    is_enabled,
    enable,
    disable,
    toggle,
    use_meritocratic_selection,
    use_cost_tracking,
    is_debug,
    get_role_model,
    set_role_model,
)


# =============================================================================
# Test Fixtures
# =============================================================================

@pytest.fixture
def fresh_config():
    """Create a fresh ConfigManager instance."""
    return ConfigManager()


@pytest.fixture
def temp_config_dir(tmp_path):
    """Create a temporary config directory."""
    return tmp_path


@pytest.fixture
def config_with_file(tmp_path):
    """Create a config with a pre-existing config file."""
    config_file = tmp_path / "config.json"
    config_file.write_text(json.dumps({
        "flags": {
            "meritocratic_selection": True,
            "debug_mode": True,
        },
        "config": {
            "default_model": "gemini",
            "min_benchmark_score": 7.5,
        }
    }))
    return tmp_path, config_file


# =============================================================================
# Feature Flag Tests
# =============================================================================

class TestFeatureFlags:
    """Tests for feature flag functionality."""

    def test_default_flags_exist(self, fresh_config):
        """All default flags should exist."""
        flags = fresh_config.list_flags()
        assert "meritocratic_selection" in flags
        assert "debug_mode" in flags
        assert "cost_tracking" in flags

    def test_meritocratic_selection_off_by_default(self, fresh_config):
        """Meritocratic selection should be OFF by default."""
        assert fresh_config.is_enabled("meritocratic_selection") is False

    def test_cost_tracking_on_by_default(self, fresh_config):
        """Cost tracking should be ON by default."""
        assert fresh_config.is_enabled("cost_tracking") is True

    def test_enable_flag(self, fresh_config):
        """Can enable a flag."""
        fresh_config.enable("meritocratic_selection")
        assert fresh_config.is_enabled("meritocratic_selection") is True

    def test_disable_flag(self, fresh_config):
        """Can disable a flag."""
        fresh_config.enable("debug_mode")
        fresh_config.disable("debug_mode")
        assert fresh_config.is_enabled("debug_mode") is False

    def test_toggle_flag(self, fresh_config):
        """Can toggle a flag."""
        initial = fresh_config.is_enabled("debug_mode")
        new_state = fresh_config.toggle("debug_mode")
        assert new_state != initial
        assert fresh_config.is_enabled("debug_mode") == new_state

    def test_toggle_returns_new_state(self, fresh_config):
        """Toggle returns the new state."""
        # Start with False
        assert fresh_config.is_enabled("debug_mode") is False
        # Toggle should return True
        result = fresh_config.toggle("debug_mode")
        assert result is True

    def test_unknown_flag_returns_false(self, fresh_config):
        """Unknown flags return False."""
        assert fresh_config.is_enabled("nonexistent_flag") is False


# =============================================================================
# Config Value Tests
# =============================================================================

class TestConfigValues:
    """Tests for config value management."""

    def test_get_default_model(self, fresh_config):
        """Can get default model."""
        assert fresh_config.get("default_model") == "claude"

    def test_get_with_default(self, fresh_config):
        """Get returns default for missing keys."""
        result = fresh_config.get("nonexistent", "fallback")
        assert result == "fallback"

    def test_set_value(self, fresh_config):
        """Can set a config value."""
        fresh_config.set("default_model", "gemini")
        assert fresh_config.get("default_model") == "gemini"

    def test_set_overrides_default(self, fresh_config):
        """Set creates override, not modifying default."""
        fresh_config.set("default_model", "groq")
        assert fresh_config.get("default_model") == "groq"
        # Default should still be claude in _config
        assert fresh_config._config["default_model"] == "claude"

    def test_get_returns_flag_value(self, fresh_config):
        """Get also works for flag names."""
        result = fresh_config.get("debug_mode")
        assert result is False  # Default value


# =============================================================================
# Role Override Tests
# =============================================================================

class TestRoleOverrides:
    """Tests for role-to-model mapping."""

    def test_no_override_by_default(self, fresh_config):
        """No role overrides by default."""
        result = fresh_config.get_role_override("architect")
        assert result is None

    def test_set_role_override(self, fresh_config):
        """Can set a role override."""
        fresh_config.set_role_override("architect", "claude")
        result = fresh_config.get_role_override("architect")
        assert result == "claude"

    def test_multiple_role_overrides(self, fresh_config):
        """Can set multiple role overrides."""
        fresh_config.set_role_override("architect", "claude")
        fresh_config.set_role_override("executor", "groq")
        fresh_config.set_role_override("critic", "gemini")

        assert fresh_config.get_role_override("architect") == "claude"
        assert fresh_config.get_role_override("executor") == "groq"
        assert fresh_config.get_role_override("critic") == "gemini"


# =============================================================================
# File Persistence Tests
# =============================================================================

class TestFilePersistence:
    """Tests for config file loading/saving."""

    def test_save_to_file(self, fresh_config, tmp_path):
        """Can save config to file."""
        config_file = tmp_path / "config.json"
        fresh_config._config["config_dir"] = str(tmp_path)

        fresh_config.enable("debug_mode")
        fresh_config.set("default_model", "groq")

        result = fresh_config.save_to_file(str(config_file))
        assert result is True
        assert config_file.exists()

        # Verify content
        data = json.loads(config_file.read_text())
        assert data["flags"]["debug_mode"] is True
        assert data["config"]["default_model"] == "groq"

    def test_load_from_file(self, fresh_config, config_with_file):
        """Can load config from file."""
        tmp_path, config_file = config_with_file

        result = fresh_config.load_from_file(str(config_file))
        assert result is True

        assert fresh_config.is_enabled("meritocratic_selection") is True
        assert fresh_config.is_enabled("debug_mode") is True
        assert fresh_config.get("default_model") == "gemini"
        assert fresh_config.get("min_benchmark_score") == 7.5

    def test_load_nonexistent_file(self, fresh_config, tmp_path):
        """Loading nonexistent file returns False."""
        result = fresh_config.load_from_file(str(tmp_path / "missing.json"))
        assert result is False

    def test_save_creates_directory(self, fresh_config, tmp_path):
        """Save creates parent directory if needed."""
        nested_dir = tmp_path / "nested" / "config"
        config_file = nested_dir / "config.json"

        result = fresh_config.save_to_file(str(config_file))
        assert result is True
        assert config_file.exists()


# =============================================================================
# Environment Variable Tests
# =============================================================================

class TestEnvironmentVariables:
    """Tests for environment variable loading."""

    def test_load_flag_from_env(self):
        """Can load flag from environment variable."""
        with patch.dict(os.environ, {"CLI_ORCH_DEBUG_MODE": "true"}):
            config = ConfigManager()
            assert config.is_enabled("debug_mode") is True

    def test_load_flag_false_from_env(self):
        """Can load false flag from environment variable."""
        with patch.dict(os.environ, {"CLI_ORCH_COST_TRACKING": "false"}):
            config = ConfigManager()
            assert config.is_enabled("cost_tracking") is False

    def test_load_string_value_from_env(self):
        """Can load string value from environment variable."""
        with patch.dict(os.environ, {"CLI_ORCH_DEFAULT_MODEL": "groq"}):
            config = ConfigManager()
            assert config.get("default_model") == "groq"

    def test_load_numeric_value_from_env(self):
        """Can load numeric value from environment variable."""
        with patch.dict(os.environ, {"CLI_ORCH_MIN_BENCHMARK_SCORE": "8.5"}):
            config = ConfigManager()
            assert config.get("min_benchmark_score") == 8.5

    def test_env_boolean_variations(self):
        """Various boolean string representations work."""
        bool_values = [
            ("true", True), ("True", True), ("TRUE", True),
            ("1", True), ("yes", True), ("on", True),
            ("false", False), ("False", False), ("FALSE", False),
            ("0", False), ("no", False), ("off", False),
        ]
        for str_val, expected in bool_values:
            with patch.dict(os.environ, {"CLI_ORCH_DEBUG_MODE": str_val}, clear=False):
                config = ConfigManager()
                assert config.is_enabled("debug_mode") is expected, f"Failed for {str_val}"


# =============================================================================
# Reset Tests
# =============================================================================

class TestReset:
    """Tests for config reset functionality."""

    def test_reset_clears_overrides(self, fresh_config):
        """Reset clears all overrides."""
        fresh_config.enable("debug_mode")
        fresh_config.set("default_model", "groq")

        fresh_config.reset()

        assert fresh_config.is_enabled("debug_mode") is False
        assert fresh_config.get("default_model") == "claude"

    def test_reset_restores_defaults(self, fresh_config):
        """Reset restores default values."""
        fresh_config.enable("meritocratic_selection")
        fresh_config.reset()

        # Should match defaults
        for flag, default in DEFAULT_FLAGS.items():
            assert fresh_config.is_enabled(flag) == default


# =============================================================================
# Convenience Function Tests
# =============================================================================

class TestConvenienceFunctions:
    """Tests for module-level convenience functions."""

    def test_use_meritocratic_selection(self):
        """use_meritocratic_selection helper works."""
        cfg = get_config()
        cfg.reset()

        assert use_meritocratic_selection() is False
        cfg.enable("meritocratic_selection")
        assert use_meritocratic_selection() is True

    def test_use_cost_tracking(self):
        """use_cost_tracking helper works."""
        cfg = get_config()
        cfg.reset()

        assert use_cost_tracking() is True  # On by default

    def test_is_debug(self):
        """is_debug helper works."""
        cfg = get_config()
        cfg.reset()

        assert is_debug() is False
        cfg.enable("debug_mode")
        assert is_debug() is True

    def test_get_role_model(self):
        """get_role_model helper works."""
        cfg = get_config()
        cfg.reset()

        assert get_role_model("architect") is None
        cfg.set_role_override("architect", "claude")
        assert get_role_model("architect") == "claude"

    def test_set_role_model(self):
        """set_role_model helper works."""
        cfg = get_config()
        cfg.reset()

        set_role_model("executor", "groq")
        assert get_role_model("executor") == "groq"


# =============================================================================
# Summary Tests
# =============================================================================

class TestSummary:
    """Tests for config summary output."""

    def test_summary_includes_flags(self, fresh_config):
        """Summary includes feature flags."""
        summary = fresh_config.summary()
        assert "Feature Flags:" in summary
        assert "meritocratic_selection" in summary

    def test_summary_includes_config(self, fresh_config):
        """Summary includes config values."""
        summary = fresh_config.summary()
        assert "Config Values:" in summary
        assert "default_model" in summary


# =============================================================================
# FeatureFlag Enum Tests
# =============================================================================

class TestFeatureFlagEnum:
    """Tests for FeatureFlag enum."""

    def test_enum_values(self):
        """Enum has expected values."""
        assert FeatureFlag.MERITOCRATIC_SELECTION.value == "meritocratic_selection"
        assert FeatureFlag.DEBUG_MODE.value == "debug_mode"
        assert FeatureFlag.COST_TRACKING.value == "cost_tracking"

    def test_enum_can_be_used_with_is_enabled(self, fresh_config):
        """Enum values work with is_enabled."""
        result = fresh_config.is_enabled(FeatureFlag.DEBUG_MODE.value)
        assert result is False


# =============================================================================
# Integration Tests
# =============================================================================

class TestIntegration:
    """Integration tests for the config system."""

    def test_full_workflow(self, tmp_path):
        """Test full config workflow: set, save, load."""
        # Create config and make changes
        config1 = ConfigManager()
        config1._config["config_dir"] = str(tmp_path)

        config1.enable("meritocratic_selection")
        config1.enable("debug_mode")
        config1.set("default_model", "gemini")
        config1.set_role_override("architect", "claude")
        config1.set_role_override("executor", "groq")

        # Save
        config_file = tmp_path / "config.json"
        config1.save_to_file(str(config_file))

        # Create new config and load
        config2 = ConfigManager()
        config2.load_from_file(str(config_file))

        # Verify
        assert config2.is_enabled("meritocratic_selection") is True
        assert config2.is_enabled("debug_mode") is True
        assert config2.get("default_model") == "gemini"
        assert config2.get_role_override("architect") == "claude"
        assert config2.get_role_override("executor") == "groq"

    def test_override_priority(self, tmp_path):
        """Test that runtime overrides take priority."""
        # Create config file
        config_file = tmp_path / "config.json"
        config_file.write_text(json.dumps({
            "flags": {"debug_mode": True},
            "config": {"default_model": "gemini"}
        }))

        # Load config
        config = ConfigManager()
        config.load_from_file(str(config_file))

        # File values should be loaded
        assert config.is_enabled("debug_mode") is True
        assert config.get("default_model") == "gemini"

        # Runtime overrides should take priority
        config.disable("debug_mode")
        config.set("default_model", "groq")

        assert config.is_enabled("debug_mode") is False
        assert config.get("default_model") == "groq"
