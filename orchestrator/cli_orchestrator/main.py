#!/usr/bin/env python3
"""
CLI Orchestrator - Main Entry Point

Run multi-agent workflows using Claude CLI, Gemini, and custom modules.

Usage:
    # Interactive mode
    python -m cli_orchestrator.main

    # Run a workflow
    python -m cli_orchestrator.main --workflow design-implement-review "Build a REST API"

    # Benchmark backends
    python -m cli_orchestrator.main --benchmark "Explain quantum computing"
"""
import argparse
import sys
from pathlib import Path

# Handle imports for both module and standalone usage
try:
    from .runners import ClaudeRunner, GeminiRunner, CustomRunner
    from .runners.custom_runner import OllamaRunner
    from .profiles import ProfileRegistry, create_default_registry
    from .orchestrator import (
        Orchestrator,
        WorkflowBuilder,
        design_implement_review,
        research_design_implement,
        debug_fix_test,
        ConvergenceCriteria
    )
    from .benchmark import Benchmarker, quality_indicators_scorer
    from .agent import Agent, AgentRole, create_agent
    from .memory_bank import MemoryBank
except ImportError:
    from runners import ClaudeRunner, GeminiRunner, CustomRunner
    from runners.custom_runner import OllamaRunner
    from profiles import ProfileRegistry, create_default_registry
    from orchestrator import (
        Orchestrator,
        WorkflowBuilder,
        design_implement_review,
        research_design_implement,
        debug_fix_test,
        ConvergenceCriteria
    )
    from benchmark import Benchmarker, quality_indicators_scorer
    from agent import Agent, AgentRole, create_agent
    from memory_bank import MemoryBank


def check_available_backends() -> dict:
    """Check which backends are available."""
    available = {}

    # Claude CLI
    claude = ClaudeRunner()
    available["claude"] = claude.is_available()

    # Gemini (API)
    from .runners.gemini_runner import check_gemini_availability
    gemini_status = check_gemini_availability()
    available["gemini-api"] = gemini_status.get("api", False)
    available["gemini-cli"] = gemini_status.get("gemini", False)

    # Ollama
    ollama = OllamaRunner()
    available["ollama"] = ollama.is_available()

    return available


def create_orchestrator(debug: bool = False) -> Orchestrator:
    """Create an orchestrator with available backends."""
    # Primary runner is Claude CLI
    claude = ClaudeRunner(debug=debug)

    if not claude.is_available():
        print("Warning: Claude CLI not available. Some features may not work.")
        print("Install with: npm install -g @anthropic-ai/claude-code")

    # Create registry with SuperClaude profiles
    registry = create_default_registry(claude)

    return Orchestrator(registry, debug=debug)


def run_interactive(orchestrator: Orchestrator):
    """Run in interactive mode."""
    print("=" * 60)
    print("CLI Orchestrator - Interactive Mode")
    print("=" * 60)
    print("\nAvailable agents:", ", ".join(orchestrator.registry.list_profiles()))
    print("\nCommands:")
    print("  workflow <agent1,agent2,...> <task>  - Run agents in sequence")
    print("  single <agent> <task>                - Run single agent")
    print("  help                                 - Show this help")
    print("  quit                                 - Exit")
    print()

    conversation = orchestrator.create_conversation("Interactive session")

    while True:
        try:
            user_input = input("You: ").strip()
        except (KeyboardInterrupt, EOFError):
            print("\nGoodbye!")
            break

        if not user_input:
            continue

        if user_input.lower() in ['quit', 'exit', 'q']:
            print("Goodbye!")
            break

        if user_input.lower() == 'help':
            print("\nAgents:", ", ".join(orchestrator.registry.list_profiles()))
            continue

        # Parse command
        parts = user_input.split(maxsplit=2)
        cmd = parts[0].lower()

        if cmd == "workflow" and len(parts) >= 3:
            agents = parts[1].split(",")
            task = parts[2]

            print(f"\nRunning workflow: {' -> '.join(agents)}")
            for agent in agents:
                print(f"\n[{agent}] Working...")
                turn = orchestrator.run_agent(conversation, agent.strip(), task)
                print(f"\n{turn.response[:500]}...")
                task = "Continue based on the above"

        elif cmd == "single" and len(parts) >= 3:
            agent = parts[1]
            task = parts[2]

            print(f"\n[{agent}] Working...")
            turn = orchestrator.run_agent(conversation, agent, task)
            print(f"\n{turn.response}")

        else:
            # Default: use architect
            print("\n[architect] Working...")
            turn = orchestrator.run_agent(conversation, "architect", user_input)
            print(f"\n{turn.response}")


def run_workflow(workflow_name: str, task: str, debug: bool = False):
    """Run a predefined workflow."""
    orchestrator = create_orchestrator(debug)

    workflows = {
        "design-implement-review": design_implement_review,
        "research-design-implement": research_design_implement,
        "debug-fix-test": debug_fix_test
    }

    if workflow_name not in workflows:
        print(f"Unknown workflow: {workflow_name}")
        print(f"Available: {', '.join(workflows.keys())}")
        return

    print(f"\nRunning workflow: {workflow_name}")
    print(f"Task: {task}")
    print("=" * 60)

    conversation = workflows[workflow_name](orchestrator, task)

    print("\n" + "=" * 60)
    print("WORKFLOW COMPLETE")
    print("=" * 60)

    for i, turn in enumerate(conversation.turns):
        print(f"\n[{i+1}. {turn.agent_name}] ({turn.execution_time:.1f}s)")
        print("-" * 40)
        # Print first 500 chars of response
        print(turn.response[:500] + "..." if len(turn.response) > 500 else turn.response)


def run_benchmark(prompt: str, ollama_model: str = "llama3.1:8b", debug: bool = False):
    """Run a benchmark across available backends."""
    print("Checking available backends...")
    available = check_available_backends()

    for backend, is_available in available.items():
        status = "✓" if is_available else "✗"
        print(f"  {status} {backend}")

    benchmarker = Benchmarker(debug=debug)

    # Add available runners
    if available.get("claude"):
        benchmarker.add_runner(ClaudeRunner(output_format="text"))

    if available.get("gemini-api"):
        from .runners import GeminiRunner
        benchmarker.add_runner(GeminiRunner(backend="api"))

    if available.get("ollama"):
        benchmarker.add_runner(OllamaRunner(model=ollama_model))

    if not benchmarker.runners:
        print("\nNo backends available for benchmarking!")
        return

    print(f"\nBenchmarking with {len(benchmarker.runners)} backends...")
    print(f"Prompt: {prompt[:50]}...")
    print()

    comparison = benchmarker.run(prompt, scorer=quality_indicators_scorer)

    print(comparison.summary())

    # Show response previews
    print("\n" + "=" * 60)
    print("RESPONSE PREVIEWS")
    print("=" * 60)

    for result in comparison.results:
        if result.success:
            print(f"\n[{result.backend}]")
            print("-" * 40)
            print(result.response[:300] + "..." if len(result.response) > 300 else result.response)


def run_moa_workflow(
    task: str,
    architect_backend: str = "gemini",
    executor_backend: str = "claude",
    max_iterations: int = 5,
    ollama_model: str = "llama3.1:8b",
    debug: bool = False
):
    """
    Run a Mixture of Agents (MoA) workflow.

    Uses agnostic Agent architecture - any backend can play any role.
    """
    print("=" * 60)
    print("MoA Workflow - Mixture of Agents")
    print("=" * 60)

    # Check available backends
    available = check_available_backends()

    # Create runners based on selection
    runners = {}

    if available.get("claude"):
        runners["claude"] = ClaudeRunner(debug=debug)
    if available.get("gemini-api"):
        try:
            runners["gemini"] = GeminiRunner(backend="api")
        except Exception:
            pass
    if available.get("ollama"):
        runners["ollama"] = OllamaRunner(model=ollama_model)

    # Fallback logic
    if architect_backend not in runners:
        architect_backend = next(iter(runners.keys())) if runners else None
    if executor_backend not in runners:
        executor_backend = next(iter(runners.keys())) if runners else None

    if not runners:
        print("Error: No backends available!")
        sys.exit(1)

    print(f"\nArchitect: {architect_backend}")
    print(f"Executor: {executor_backend}")
    print(f"Task: {task}")
    print()

    # Create agnostic agents
    architect = create_agent(
        name="architect",
        runner=runners[architect_backend],
        role=AgentRole.ARCHITECT
    )

    executor = create_agent(
        name="executor",
        runner=runners[executor_backend],
        role=AgentRole.EXECUTOR
    )

    # Create orchestrator and run
    orchestrator = Orchestrator(debug=debug)
    convergence = ConvergenceCriteria(max_iterations=max_iterations)

    conversation = orchestrator.run_moa_workflow(
        goal=task,
        architect=architect,
        executor=executor,
        convergence=convergence
    )

    # Print results
    print("\n" + "=" * 60)
    print(f"WORKFLOW {'COMPLETE' if conversation.status == 'completed' else 'FAILED'}")
    print(f"Iterations: {conversation.metadata.get('iterations', 'N/A')}")
    print("=" * 60)

    for i, turn in enumerate(conversation.turns):
        print(f"\n[{i+1}. {turn.agent_name}] ({turn.execution_time:.1f}s)")
        print("-" * 40)
        print(turn.response[:500] + "..." if len(turn.response) > 500 else turn.response)


def show_status(debug: bool = False):
    """Show current orchestrator status."""
    print("=" * 60)
    print("CLI Orchestrator Status")
    print("=" * 60)

    # Check backends
    print("\nAvailable Backends:")
    available = check_available_backends()
    for backend, is_available in available.items():
        status = "✓" if is_available else "✗"
        print(f"  {status} {backend}")

    # Check memory bank
    memory_bank = MemoryBank()
    print("\nMemory Bank:")
    if memory_bank.active_context.exists():
        print("  ✓ Active context found")
        context = memory_bank.read_context()
        print(f"  Context size: {len(context)} chars (~{len(context)//4} tokens)")

        # Show snippet of active context
        if memory_bank.active_context.exists():
            active = memory_bank.active_context.read_text()
            lines = active.split('\n')[:10]
            print("\n  Recent context:")
            for line in lines:
                print(f"    {line[:60]}")
    else:
        print("  ✗ No active context (start with --moa)")

    print()


def resume_workflow(debug: bool = False):
    """Resume a paused/failed workflow from memory bank state."""
    memory_bank = MemoryBank()

    if not memory_bank.active_context.exists():
        print("Error: No workflow to resume. Start with --moa first.")
        sys.exit(1)

    print("=" * 60)
    print("Resuming Workflow from Memory Bank")
    print("=" * 60)

    # Read current context
    context = memory_bank.read_context()
    print(f"\nLoaded context: {len(context)} chars")

    # Extract goal from product context
    goal = "Continue previous task"  # Default
    if memory_bank.product_context.exists():
        product = memory_bank.product_context.read_text()
        for line in product.split('\n'):
            if line.strip() and not line.startswith('#'):
                goal = line.strip()
                break

    print(f"Goal: {goal}")
    print()

    # Create agents with available backends
    available = check_available_backends()
    runners = {}
    if available.get("claude"):
        runners["claude"] = ClaudeRunner(debug=debug)
    if available.get("gemini-api"):
        try:
            runners["gemini"] = GeminiRunner(backend="api")
        except Exception:
            pass

    if not runners:
        print("Error: No backends available!")
        sys.exit(1)

    # Use first available for both (user can customize)
    backend = next(iter(runners.keys()))
    architect = create_agent("architect", runners[backend], AgentRole.ARCHITECT)
    executor = create_agent("executor", runners[backend], AgentRole.EXECUTOR)

    # Run with existing context injected
    orchestrator = Orchestrator(debug=debug, memory_bank=memory_bank)

    conversation = orchestrator.run_moa_workflow(
        goal=goal,
        architect=architect,
        executor=executor
    )

    print(f"\nWorkflow {'completed' if conversation.status == 'completed' else 'stopped'}")


def main():
    parser = argparse.ArgumentParser(
        description="CLI Orchestrator - Multi-Agent Workflow Orchestration",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Run MoA workflow (Plan → Execute → Review loop)
  python main.py --moa "Build a REST API for user management"

  # Resume interrupted workflow
  python main.py --resume

  # Check status
  python main.py --status

  # Specify backends
  python main.py --moa "Build API" --architect gemini --executor claude

  # Use Ollama (requires `ollama serve` running)
  python main.py --moa "Explain Python" --architect ollama --executor ollama
  python main.py --moa "Task" --executor ollama --model mistral
        """
    )

    # Mode selection (mutually exclusive)
    mode_group = parser.add_mutually_exclusive_group()
    mode_group.add_argument(
        "--workflow", "-w",
        choices=["design-implement-review", "research-design-implement", "debug-fix-test"],
        help="Run a predefined workflow (legacy)"
    )
    mode_group.add_argument(
        "--moa",
        action="store_true",
        help="Run Mixture of Agents workflow (Plan→Execute→Review loop)"
    )
    mode_group.add_argument(
        "--resume",
        action="store_true",
        help="Resume interrupted workflow from memory bank"
    )
    mode_group.add_argument(
        "--status",
        action="store_true",
        help="Show orchestrator status and memory bank state"
    )
    mode_group.add_argument(
        "--benchmark", "-b",
        action="store_true",
        help="Run benchmark mode"
    )
    mode_group.add_argument(
        "--check",
        action="store_true",
        help="Check available backends"
    )

    # MoA configuration
    parser.add_argument(
        "--architect",
        default="gemini",
        help="Backend for architect agent (default: gemini)"
    )
    parser.add_argument(
        "--executor",
        default="claude",
        help="Backend for executor agent (default: claude)"
    )
    parser.add_argument(
        "--max-iterations",
        type=int,
        default=5,
        help="Maximum MoA iterations (default: 5)"
    )
    parser.add_argument(
        "--model",
        default="llama3.1:8b",
        help="Ollama model to use when backend=ollama (default: llama3.1:8b)"
    )

    # Common options
    parser.add_argument(
        "--debug", "-d",
        action="store_true",
        help="Enable debug output"
    )
    parser.add_argument(
        "task",
        nargs="?",
        help="Task or prompt to process"
    )

    args = parser.parse_args()

    # Handle modes
    if args.check:
        print("Checking available backends...")
        available = check_available_backends()
        for backend, is_available in available.items():
            status = "✓ Available" if is_available else "✗ Not available"
            print(f"  {backend}: {status}")
        return

    if args.status:
        show_status(args.debug)
        return

    if args.resume:
        resume_workflow(args.debug)
        return

    if args.moa:
        if not args.task:
            print("Error: --moa requires a task description")
            print("Usage: python main.py --moa \"Your task here\"")
            sys.exit(1)
        run_moa_workflow(
            task=args.task,
            architect_backend=args.architect,
            executor_backend=args.executor,
            max_iterations=args.max_iterations,
            ollama_model=args.model,
            debug=args.debug
        )
        return

    if args.benchmark:
        if not args.task:
            print("Error: --benchmark requires a prompt")
            sys.exit(1)
        run_benchmark(args.task, ollama_model=args.model, debug=args.debug)
        return

    if args.workflow:
        if not args.task:
            print("Error: --workflow requires a task description")
            sys.exit(1)
        run_workflow(args.workflow, args.task, args.debug)
        return

    # Default: interactive mode
    orchestrator = create_orchestrator(args.debug)
    run_interactive(orchestrator)


if __name__ == "__main__":
    main()
