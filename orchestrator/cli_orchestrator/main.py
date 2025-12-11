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
        debug_fix_test
    )
    from .benchmark import Benchmarker, quality_indicators_scorer
except ImportError:
    from runners import ClaudeRunner, GeminiRunner, CustomRunner
    from runners.custom_runner import OllamaRunner
    from profiles import ProfileRegistry, create_default_registry
    from orchestrator import (
        Orchestrator,
        WorkflowBuilder,
        design_implement_review,
        research_design_implement,
        debug_fix_test
    )
    from benchmark import Benchmarker, quality_indicators_scorer


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


def run_benchmark(prompt: str, debug: bool = False):
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
        benchmarker.add_runner(OllamaRunner(model="llama3.1:8b"))

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


def main():
    parser = argparse.ArgumentParser(
        description="CLI Orchestrator - Multi-Agent Workflow Orchestration"
    )
    parser.add_argument(
        "--workflow", "-w",
        choices=["design-implement-review", "research-design-implement", "debug-fix-test"],
        help="Run a predefined workflow"
    )
    parser.add_argument(
        "--benchmark", "-b",
        action="store_true",
        help="Run benchmark mode"
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check available backends"
    )
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

    if args.check:
        print("Checking available backends...")
        available = check_available_backends()
        for backend, is_available in available.items():
            status = "✓ Available" if is_available else "✗ Not available"
            print(f"  {backend}: {status}")
        return

    if args.benchmark:
        if not args.task:
            print("Error: --benchmark requires a prompt")
            sys.exit(1)
        run_benchmark(args.task, args.debug)
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
