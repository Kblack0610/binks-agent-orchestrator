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
    PROMPTS,
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
    "PROMPTS",
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
]
