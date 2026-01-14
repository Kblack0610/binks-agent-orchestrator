"""
Feature Flag Configuration System

Centralized configuration for CLI Orchestrator features.
Supports environment variables, config files, and programmatic overrides.

Usage:
    from cli_orchestrator import config

    # Check a feature flag
    if config.is_enabled("meritocratic_selection"):
        # Use benchmark-based model selection
        pass

    # Get a config value
    default_model = config.get("default_model")

    # Override at runtime
    config.set("debug_mode", True)

    # Load from file
    config.load_from_file("~/.cli_orchestrator/config.json")
"""
import os
import json
from pathlib import Path
from typing import Any, Dict, Optional, List
from dataclasses import dataclass, field
from enum import Enum


# =============================================================================
# Feature Definitions
# =============================================================================

class FeatureFlag(Enum):
    """Available feature flags."""

    # Scoring & Evaluation (separate from selection!)
    RESPONSE_SCORING = "response_scoring"      # Score every response (Gatekeeper + Judge)
    AUTO_BENCHMARK = "auto_benchmark"          # Run benchmarks automatically
    STRICT_GATEKEEPER = "strict_gatekeeper"    # Stricter heuristic validation

    # Model Selection (uses scores, but separate concern)
    MERITOCRATIC_SELECTION = "meritocratic_selection"  # Select models based on scores
    COST_TRACKING = "cost_tracking"            # Track costs per model
    AUTO_FALLBACK = "auto_fallback"            # Auto fallback on failure

    # Debugging & Logging
    DEBUG_MODE = "debug_mode"
    VERBOSE_LOGGING = "verbose_logging"
    LOG_RESPONSES = "log_responses"

    # Agent Behavior
    MULTI_AGENT_MODE = "multi_agent_mode"
    AGENT_MEMORY = "agent_memory"

    # Experimental
    EXPERIMENTAL_FEATURES = "experimental_features"


# Default values for all flags
DEFAULT_FLAGS: Dict[str, bool] = {
    # Scoring & Evaluation - OFF by default (enable to score responses)
    "response_scoring": False,      # Score every response with Gatekeeper + Judge
    "auto_benchmark": False,        # Run benchmarks automatically
    "strict_gatekeeper": False,     # Stricter heuristic validation

    # Model Selection - OFF by default until benchmarks are run
    "meritocratic_selection": False,  # Select models based on scores
    "cost_tracking": True,            # Track costs per model
    "auto_fallback": False,           # Auto fallback on failure

    # Debugging - OFF by default
    "debug_mode": False,
    "verbose_logging": False,
    "log_responses": False,

    # Agent Behavior
    "multi_agent_mode": True,
    "agent_memory": False,

    # Experimental - OFF by default
    "experimental_features": False,
}

# =============================================================================
# Default Role-to-Model Mapping
# =============================================================================
# Each role has a default model/runner assignment based on task complexity.
# Priority: role_overrides > meritocratic_selection > DEFAULT_ROLE_MODELS
#
# Available runners: claude, gemini, groq, openrouter
# =============================================================================

DEFAULT_ROLE_MODELS: Dict[str, str] = {
    # High-complexity roles - need best models
    "judge": "claude",          # Quality evaluation requires highest capability
    "architect": "claude",      # Complex design work needs strong reasoning
    "gatekeeper": "claude",     # Quality gating needs good judgment

    # Medium-complexity roles - good free models work well
    "critic": "gemini",         # Code review - Gemini is good and free via CLI
    "planner": "gemini",        # Requirements gathering
    "executor": "gemini",       # Implementation - Gemini is fast and free
    "researcher": "gemini",     # Information gathering - Gemini excels here

    # Lower-complexity roles - fast/free models preferred
    # Note: Using gemini for now since groq requires API key setup
    "triage": "gemini",         # Simple routing decision
    "debugger": "gemini",       # Pattern matching, fast iteration
    "tester": "gemini",         # Test generation
    "verifier": "gemini",       # Running checks
    "documenter": "gemini",     # Documentation writing
}


# Default config values (non-boolean settings)
DEFAULT_CONFIG: Dict[str, Any] = {
    # Model defaults
    "default_model": "claude",
    "judge_model": "claude",
    "min_benchmark_score": 6.0,

    # Paths
    "config_dir": "~/.cli_orchestrator",
    "scores_file": "model_scores.json",
    "config_file": "config.json",

    # Evaluation weights
    "gatekeeper_weight": 0.3,
    "judge_weight": 0.7,

    # Role-to-model mapping (uses DEFAULT_ROLE_MODELS, can be overridden)
    "role_models": DEFAULT_ROLE_MODELS.copy(),

    # Role overrides (explicit assignments that override everything)
    "role_overrides": {},

    # Cost tracking
    "cost_per_1k_tokens": {
        "groq": 0.0,  # Free tier
        "openrouter": 0.0,  # Free models
        "claude": 0.0,  # CLI membership
        "gemini": 0.0,  # CLI membership
    },
}


# =============================================================================
# Configuration Manager
# =============================================================================

@dataclass
class ConfigManager:
    """
    Centralized configuration manager.

    Priority (highest to lowest):
    1. Runtime overrides (set programmatically)
    2. Environment variables (CLI_ORCH_*)
    3. Config file (~/.cli_orchestrator/config.json)
    4. Default values
    """

    _flags: Dict[str, bool] = field(default_factory=lambda: DEFAULT_FLAGS.copy())
    _config: Dict[str, Any] = field(default_factory=lambda: DEFAULT_CONFIG.copy())
    _overrides: Dict[str, Any] = field(default_factory=dict)
    _loaded_from_file: bool = False

    def __post_init__(self):
        """Initialize by loading from environment and file."""
        self._load_from_environment()
        self._auto_load_config_file()

    # -------------------------------------------------------------------------
    # Feature Flags
    # -------------------------------------------------------------------------

    def is_enabled(self, flag: str) -> bool:
        """
        Check if a feature flag is enabled.

        Args:
            flag: Flag name (e.g., "meritocratic_selection")

        Returns:
            True if enabled, False otherwise
        """
        # Check overrides first
        if flag in self._overrides:
            return bool(self._overrides[flag])

        # Then check flags
        return self._flags.get(flag, False)

    def enable(self, flag: str) -> None:
        """Enable a feature flag."""
        self._overrides[flag] = True

    def disable(self, flag: str) -> None:
        """Disable a feature flag."""
        self._overrides[flag] = False

    def toggle(self, flag: str) -> bool:
        """Toggle a feature flag. Returns new state."""
        current = self.is_enabled(flag)
        self._overrides[flag] = not current
        return not current

    # -------------------------------------------------------------------------
    # Config Values
    # -------------------------------------------------------------------------

    def get(self, key: str, default: Any = None) -> Any:
        """
        Get a config value.

        Args:
            key: Config key
            default: Default value if not found

        Returns:
            Config value
        """
        # Check overrides first
        if key in self._overrides:
            return self._overrides[key]

        # Check if it's a flag
        if key in self._flags:
            return self._flags[key]

        # Check config
        return self._config.get(key, default)

    def set(self, key: str, value: Any) -> None:
        """Set a config value (runtime override)."""
        self._overrides[key] = value

    def get_role_override(self, role: str) -> Optional[str]:
        """Get explicit model override for a role."""
        overrides = self.get("role_overrides", {})
        return overrides.get(role)

    def set_role_override(self, role: str, model: str) -> None:
        """Set explicit model for a role."""
        if "role_overrides" not in self._overrides:
            self._overrides["role_overrides"] = self._config.get("role_overrides", {}).copy()
        self._overrides["role_overrides"][role] = model

    # -------------------------------------------------------------------------
    # File Operations
    # -------------------------------------------------------------------------

    def _get_config_path(self) -> Path:
        """Get the config file path."""
        config_dir = Path(self._config["config_dir"]).expanduser()
        return config_dir / self._config["config_file"]

    def _auto_load_config_file(self) -> None:
        """Auto-load config file if it exists."""
        config_path = self._get_config_path()
        if config_path.exists():
            self.load_from_file(config_path)

    def load_from_file(self, path: Optional[str] = None) -> bool:
        """
        Load configuration from JSON file.

        Args:
            path: Path to config file (uses default if None)

        Returns:
            True if loaded successfully
        """
        if path is None:
            config_path = self._get_config_path()
        else:
            config_path = Path(path).expanduser()

        if not config_path.exists():
            return False

        try:
            with open(config_path, "r") as f:
                data = json.load(f)

            # Load flags
            if "flags" in data:
                for key, value in data["flags"].items():
                    if key in self._flags:
                        self._flags[key] = bool(value)

            # Load config values
            if "config" in data:
                for key, value in data["config"].items():
                    self._config[key] = value

            self._loaded_from_file = True
            return True

        except (json.JSONDecodeError, IOError):
            return False

    def save_to_file(self, path: Optional[str] = None) -> bool:
        """
        Save current configuration to JSON file.

        Args:
            path: Path to config file (uses default if None)

        Returns:
            True if saved successfully
        """
        if path is None:
            config_path = self._get_config_path()
        else:
            config_path = Path(path).expanduser()

        # Ensure directory exists
        config_path.parent.mkdir(parents=True, exist_ok=True)

        # Merge overrides into flags/config for saving
        merged_flags = self._flags.copy()
        merged_config = self._config.copy()

        for key, value in self._overrides.items():
            if key in merged_flags:
                merged_flags[key] = value
            else:
                merged_config[key] = value

        data = {
            "flags": merged_flags,
            "config": merged_config,
        }

        try:
            with open(config_path, "w") as f:
                json.dump(data, f, indent=2)
            return True
        except IOError:
            return False

    # -------------------------------------------------------------------------
    # Environment Variables
    # -------------------------------------------------------------------------

    def _load_from_environment(self) -> None:
        """Load config from environment variables (CLI_ORCH_*)."""
        prefix = "CLI_ORCH_"

        for key in os.environ:
            if key.startswith(prefix):
                config_key = key[len(prefix):].lower()
                value = os.environ[key]

                # Parse boolean values
                if value.lower() in ("true", "1", "yes", "on"):
                    parsed_value = True
                elif value.lower() in ("false", "0", "no", "off"):
                    parsed_value = False
                else:
                    # Try to parse as number
                    try:
                        parsed_value = float(value) if "." in value else int(value)
                    except ValueError:
                        parsed_value = value

                # Set in appropriate place
                if config_key in self._flags:
                    self._flags[config_key] = bool(parsed_value)
                else:
                    self._config[config_key] = parsed_value

    # -------------------------------------------------------------------------
    # Utilities
    # -------------------------------------------------------------------------

    def list_flags(self) -> Dict[str, bool]:
        """List all feature flags with current values."""
        result = self._flags.copy()
        for key in result:
            if key in self._overrides:
                result[key] = self._overrides[key]
        return result

    def list_config(self) -> Dict[str, Any]:
        """List all config values."""
        result = self._config.copy()
        result.update(self._overrides)
        return result

    def reset(self) -> None:
        """Reset all overrides to defaults."""
        self._overrides.clear()
        self._flags = DEFAULT_FLAGS.copy()
        self._config = DEFAULT_CONFIG.copy()

    def summary(self) -> str:
        """Get a human-readable summary of current config."""
        lines = ["CLI Orchestrator Configuration", "=" * 40, "", "Feature Flags:"]

        for flag, enabled in sorted(self.list_flags().items()):
            status = "ON" if enabled else "OFF"
            lines.append(f"  {flag}: {status}")

        lines.extend(["", "Config Values:"])
        for key, value in sorted(self.list_config().items()):
            if key not in self._flags:
                lines.append(f"  {key}: {value}")

        return "\n".join(lines)


# =============================================================================
# Global Instance
# =============================================================================

# Singleton config manager
_config_manager: Optional[ConfigManager] = None


def get_config() -> ConfigManager:
    """Get the global config manager instance."""
    global _config_manager
    if _config_manager is None:
        _config_manager = ConfigManager()
    return _config_manager


# =============================================================================
# Convenience Functions
# =============================================================================

def is_enabled(flag: str) -> bool:
    """Check if a feature flag is enabled."""
    return get_config().is_enabled(flag)


def enable(flag: str) -> None:
    """Enable a feature flag."""
    get_config().enable(flag)


def disable(flag: str) -> None:
    """Disable a feature flag."""
    get_config().disable(flag)


def toggle(flag: str) -> bool:
    """Toggle a feature flag. Returns new state."""
    return get_config().toggle(flag)


def get(key: str, default: Any = None) -> Any:
    """Get a config value."""
    return get_config().get(key, default)


def set(key: str, value: Any) -> None:
    """Set a config value."""
    get_config().set(key, value)


def save() -> bool:
    """Save current config to file."""
    return get_config().save_to_file()


def load(path: Optional[str] = None) -> bool:
    """Load config from file."""
    return get_config().load_from_file(path)


def summary() -> str:
    """Get config summary."""
    return get_config().summary()


def list_flags() -> Dict[str, bool]:
    """List all feature flags."""
    return get_config().list_flags()


# =============================================================================
# Feature-Specific Helpers
# =============================================================================

def use_response_scoring() -> bool:
    """Check if response scoring is enabled (Gatekeeper + Judge evaluation)."""
    return is_enabled("response_scoring")


def use_meritocratic_selection() -> bool:
    """Check if meritocratic model selection is enabled (uses scores to pick models)."""
    return is_enabled("meritocratic_selection")


def use_cost_tracking() -> bool:
    """Check if cost tracking is enabled."""
    return is_enabled("cost_tracking")


def is_debug() -> bool:
    """Check if debug mode is enabled."""
    return is_enabled("debug_mode")


def get_role_model(role: str) -> Optional[str]:
    """Get the configured model for a role (if any override exists)."""
    return get_config().get_role_override(role)


def set_role_model(role: str, model: str) -> None:
    """Set the model to use for a specific role."""
    get_config().set_role_override(role, model)


def get_model_for_role(role: str) -> str:
    """
    Get the model name to use for a role.

    Priority:
    1. Explicit role overrides
    2. DEFAULT_ROLE_MODELS
    3. Falls back to default_model config

    Note: meritocratic_selection can override this at the agent level,
    but this function returns the default/configured model.

    Args:
        role: Role name (e.g., "executor", "architect", "judge")

    Returns:
        Model name (e.g., "claude", "gemini", "groq", "openrouter")
    """
    # Check explicit override first
    override = get_config().get_role_override(role)
    if override:
        return override

    # Check default role models
    role_models = get_config().get("role_models", DEFAULT_ROLE_MODELS)
    if role in role_models:
        return role_models[role]

    # Fall back to default model
    return get_config().get("default_model", "claude")


def get_runner_for_role(role: str):
    """
    Get a runner instance for the specified role.

    This creates the appropriate CLIRunner based on the role-to-model mapping.

    Args:
        role: Role name (e.g., "executor", "architect", "judge")

    Returns:
        CLIRunner instance (ClaudeRunner, GeminiRunner, GroqRunner, OpenRouterRunner, or FactoryRunner)
    """
    # Import here to avoid circular imports
    from cli_orchestrator.runners import (
        ClaudeRunner, GeminiRunner, GroqRunner, OpenRouterRunner, FactoryRunner
    )
    import shutil

    model = get_model_for_role(role)

    if model == "claude":
        return ClaudeRunner()
    elif model == "gemini":
        # Prefer CLI if available, fall back to API
        if shutil.which("gemini"):
            return GeminiRunner(backend="gemini")  # Use CLI
        else:
            return GeminiRunner(backend="api")  # Fall back to API
    elif model == "groq":
        return GroqRunner()
    elif model == "openrouter":
        return OpenRouterRunner()
    elif model == "factory":
        return FactoryRunner()
    else:
        # Default to Claude for unknown models
        return ClaudeRunner()
