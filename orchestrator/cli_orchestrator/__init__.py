# CLI Orchestrator - Multi-Agent Headless Orchestration
# Wraps Claude CLI, Gemini CLI, and custom modules for agent collaboration

from .agent import (
    Agent,
    AgentRole,
    AgentResponse,
    create_agent,
    create_architect,
    create_executor,
    create_critic,
    create_researcher,
    create_verifier,
    create_triage,
    create_planner,
    create_gatekeeper,
    create_judge,
    PROMPTS,
    WORKFLOWS,
    ROLE_DESCRIPTIONS,
)

from .model_evaluator import (
    ModelEvaluator,
    Gatekeeper,
    Judge,
    ScoreStore,
    ModelSelector,
    ROLE_BENCHMARKS,
    list_roles,
    get_role_benchmark,
    quick_evaluate,
)

from .benchmark import (
    Benchmarker,
    BenchmarkResult,
    BenchmarkComparison,
)

from . import config
from .config import (
    ConfigManager,
    FeatureFlag,
    get_config,
    is_enabled,
    enable,
    disable,
    toggle,
    use_response_scoring,
    use_meritocratic_selection,
    use_cost_tracking,
    is_debug,
    get_role_model,
    set_role_model,
)

__all__ = [
    # Agent
    "Agent",
    "AgentRole",
    "AgentResponse",
    "create_agent",
    "create_architect",
    "create_executor",
    "create_critic",
    "create_researcher",
    "create_verifier",
    "create_triage",
    "create_planner",
    "create_gatekeeper",
    "create_judge",
    "PROMPTS",
    "WORKFLOWS",
    "ROLE_DESCRIPTIONS",
    # Model Evaluator
    "ModelEvaluator",
    "Gatekeeper",
    "Judge",
    "ScoreStore",
    "ModelSelector",
    "ROLE_BENCHMARKS",
    "list_roles",
    "get_role_benchmark",
    "quick_evaluate",
    # Benchmark
    "Benchmarker",
    "BenchmarkResult",
    "BenchmarkComparison",
    # Config
    "config",
    "ConfigManager",
    "FeatureFlag",
    "get_config",
    "is_enabled",
    "enable",
    "disable",
    "toggle",
    "use_response_scoring",
    "use_meritocratic_selection",
    "use_cost_tracking",
    "is_debug",
    "get_role_model",
    "set_role_model",
]
