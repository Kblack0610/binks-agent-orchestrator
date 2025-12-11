#!/usr/bin/env python3
"""
Multi-Agent Demo - Headless Claude Sessions with SuperClaude Profiles

This demonstrates how to:
1. Run multiple Claude sessions with different SuperClaude profiles
2. Pass context between agents (architect -> implementer -> reviewer)
3. Integrate Gemini for comparison/critique
4. Use custom Python modules in the workflow

Usage:
    python multi_agent_demo.py "Build a Python script that benchmarks 3 APIs"
"""
import sys
import os

# Add parent to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from runners import ClaudeRunner, GeminiRunner, CustomRunner
from runners.custom_runner import OllamaRunner
from profiles import ProfileRegistry, AgentProfile, AgentRole, create_default_registry
from orchestrator import Orchestrator, WorkflowBuilder


def demo_basic_workflow():
    """
    Demo 1: Basic multi-agent workflow

    Architect designs -> Implementer codes -> Reviewer checks
    """
    print("\n" + "=" * 60)
    print("DEMO 1: Basic Design-Implement-Review Workflow")
    print("=" * 60)

    # Create Claude runner
    claude = ClaudeRunner(debug=True)

    if not claude.is_available():
        print("Claude CLI not available. Install with:")
        print("  npm install -g @anthropic-ai/claude-code")
        return

    # Create registry with SuperClaude profiles
    registry = create_default_registry(claude)

    # Create orchestrator
    orchestrator = Orchestrator(registry, debug=True)

    # Build and run workflow
    task = "Create a Python function to validate email addresses"

    conversation = (WorkflowBuilder(orchestrator)
        .goal(task)
        .then("architect", f"Design a robust solution for: {task}")
        .then("implementer", "Implement the designed solution with tests")
        .then("reviewer", "Review the implementation for edge cases and best practices")
        .build())

    # Print results
    print("\n" + "=" * 60)
    print("WORKFLOW RESULTS")
    print("=" * 60)

    for turn in conversation.turns:
        print(f"\n[{turn.agent_name}] ({turn.execution_time:.1f}s)")
        print("-" * 40)
        print(turn.response[:1000])
        if len(turn.response) > 1000:
            print("... (truncated)")


def demo_multi_backend():
    """
    Demo 2: Compare Claude vs Gemini on the same task

    Useful for benchmarking or getting multiple perspectives.
    """
    print("\n" + "=" * 60)
    print("DEMO 2: Multi-Backend Comparison (Claude vs Gemini)")
    print("=" * 60)

    from benchmark import Benchmarker, quality_indicators_scorer

    benchmarker = Benchmarker(debug=True)

    # Add Claude
    claude = ClaudeRunner(output_format="text")
    if claude.is_available():
        benchmarker.add_runner(claude)

    # Add Gemini (if API key is set)
    gemini = GeminiRunner(backend="api")
    if gemini.is_available():
        benchmarker.add_runner(gemini)

    if not benchmarker.runners:
        print("No backends available!")
        return

    prompt = "What are the key considerations when designing a microservices architecture?"

    print(f"\nPrompt: {prompt}")
    print(f"Backends: {[r.name for r in benchmarker.runners]}")

    comparison = benchmarker.run(prompt, scorer=quality_indicators_scorer)

    print(comparison.summary())


def demo_custom_module():
    """
    Demo 3: Integrate a custom Python function as an agent

    Shows how to add your own code to the orchestration.
    """
    print("\n" + "=" * 60)
    print("DEMO 3: Custom Module Integration")
    print("=" * 60)

    # Define a custom "agent" function
    def code_linter(prompt: str, **kwargs) -> str:
        """
        A simple custom agent that does static analysis.
        In reality, you'd integrate pylint, mypy, etc.
        """
        analysis = []
        analysis.append("Static Analysis Report:")
        analysis.append("-" * 40)

        # Simple checks (replace with real linting)
        if "import os" in prompt.lower():
            analysis.append("⚠️  Uses os module - check for path injection")

        if "eval(" in prompt.lower() or "exec(" in prompt.lower():
            analysis.append("🚨 Uses eval/exec - security risk!")

        if "password" in prompt.lower() and "=" in prompt:
            analysis.append("🚨 Hardcoded password detected!")

        if not analysis[2:]:
            analysis.append("✅ No obvious issues found")

        return "\n".join(analysis)

    # Create custom runner from the function
    linter = CustomRunner.from_callable(code_linter, name="code-linter")

    # Create profile for the custom agent
    linter_profile = AgentProfile(
        name="linter",
        role=AgentRole.REVIEWER,
        runner=linter,
        description="Static code analyzer"
    )

    # Create registry and add custom profile
    claude = ClaudeRunner()
    registry = create_default_registry(claude)
    registry.register(linter_profile)

    # Create orchestrator
    orchestrator = Orchestrator(registry, debug=True)

    # Run a workflow that includes the custom linter
    code_to_analyze = '''
def authenticate(username, password):
    import os
    admin_password = "secret123"
    if password == admin_password:
        return True
    return False
'''

    print("\nCode to analyze:")
    print(code_to_analyze)

    if claude.is_available():
        conversation = (WorkflowBuilder(orchestrator)
            .goal("Analyze and fix security issues in this code")
            .then("linter", f"Analyze this code:\n{code_to_analyze}")
            .then("troubleshooter", "Based on the linter report, identify all security issues")
            .then("implementer", "Provide a secure version of this code")
            .build())

        for turn in conversation.turns:
            print(f"\n[{turn.agent_name}]")
            print("-" * 40)
            print(turn.response)
    else:
        # Just run the linter
        result = linter.run(code_to_analyze)
        print(f"\n[code-linter]")
        print(result.content)


def demo_loop_conversation():
    """
    Demo 4: Iterative conversation loop between agents

    Useful for refining solutions through back-and-forth.
    """
    print("\n" + "=" * 60)
    print("DEMO 4: Iterative Agent Loop")
    print("=" * 60)

    claude = ClaudeRunner()

    if not claude.is_available():
        print("Claude CLI not available")
        return

    registry = create_default_registry(claude)
    orchestrator = Orchestrator(registry, debug=True, max_turns=6)

    # Define stop condition
    def is_satisfied(conversation) -> bool:
        """Stop when we have 3+ turns or see approval."""
        if len(conversation.turns) >= 3:
            return True
        last = conversation.get_last_response()
        if last and ("approved" in last.lower() or "lgtm" in last.lower()):
            return True
        return False

    conversation = orchestrator.run_loop(
        goal="Design and refine a caching strategy",
        agents=["architect", "reviewer"],
        initial_prompt="Design a caching strategy for a high-traffic web API",
        stop_condition=is_satisfied,
        max_iterations=3
    )

    print("\n" + "=" * 60)
    print("LOOP RESULTS")
    print("=" * 60)

    for i, turn in enumerate(conversation.turns):
        print(f"\n[Turn {i+1}: {turn.agent_name}]")
        print("-" * 40)
        print(turn.response[:500])


def main():
    """Run all demos or a specific one."""
    if len(sys.argv) > 1:
        demo_num = sys.argv[1]
        demos = {
            "1": demo_basic_workflow,
            "2": demo_multi_backend,
            "3": demo_custom_module,
            "4": demo_loop_conversation,
        }
        if demo_num in demos:
            demos[demo_num]()
        else:
            print(f"Unknown demo: {demo_num}")
            print("Available: 1, 2, 3, 4")
    else:
        print("CLI Orchestrator Demos")
        print("=" * 60)
        print("\nUsage: python multi_agent_demo.py [demo_number]")
        print("\nDemos:")
        print("  1 - Basic Design-Implement-Review workflow")
        print("  2 - Multi-backend comparison (Claude vs Gemini)")
        print("  3 - Custom Python module integration")
        print("  4 - Iterative agent conversation loop")
        print("\nOr run without arguments to see this help.")

        # Quick check of available backends
        print("\n" + "-" * 60)
        print("Backend Status:")

        claude = ClaudeRunner()
        print(f"  Claude CLI: {'✓' if claude.is_available() else '✗'}")

        gemini = GeminiRunner(backend="api")
        print(f"  Gemini API: {'✓' if gemini.is_available() else '✗'}")

        ollama = OllamaRunner()
        print(f"  Ollama:     {'✓' if ollama.is_available() else '✗'}")


if __name__ == "__main__":
    main()
