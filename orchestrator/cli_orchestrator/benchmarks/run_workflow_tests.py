#!/usr/bin/env python3
"""
Workflow Test Runner

Runs test projects through the triage and workflow system, collecting scores.

Usage:
    python run_workflow_tests.py                    # Run all tests
    python run_workflow_tests.py quick_01           # Run specific test
    python run_workflow_tests.py --workflow quick   # Run all quick tests
    python run_workflow_tests.py --dry-run          # Just test triage routing
"""

import sys
import os
import json
import time
from datetime import datetime
from pathlib import Path
from dataclasses import dataclass, field, asdict
from typing import Optional, List, Dict, Any

# Add the orchestrator to path
ORCHESTRATOR_PATH = "/home/kblack0610/dev/home/binks-agent-orchestrator/orchestrator"
sys.path.insert(0, ORCHESTRATOR_PATH)

from cli_orchestrator import (
    create_agent, create_triage,
    AgentRole, WORKFLOWS, ROLE_DESCRIPTIONS,
    config,
)
from cli_orchestrator.model_evaluator import Gatekeeper, Judge, ScoreStore
from cli_orchestrator.runners import ClaudeRunner, GeminiRunner, GroqRunner, OpenRouterRunner
from cli_orchestrator.config import get_runner_for_role, get_model_for_role, DEFAULT_ROLE_MODELS

from test_projects import TEST_PROJECTS, get_project_by_id, get_projects_by_workflow


@dataclass
class TestResult:
    """Result of running a single test project."""
    project_id: str
    project_name: str
    task: str

    # Triage results
    triage_workflow: str
    expected_workflow: str
    triage_correct: bool
    triage_reasoning: str

    # Execution results
    executed: bool = False
    response: str = ""
    execution_time: float = 0.0

    # Scoring results
    gatekeeper_score: float = 0.0
    gatekeeper_passed: bool = False
    judge_score: float = 0.0
    final_score: float = 0.0

    # Metadata
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())
    backend: str = ""
    error: Optional[str] = None


@dataclass
class TestSummary:
    """Summary of all test runs."""
    total_tests: int = 0
    triage_correct: int = 0
    executed: int = 0
    passed_gatekeeper: int = 0
    avg_score: float = 0.0
    results: List[TestResult] = field(default_factory=list)

    def add_result(self, result: TestResult):
        self.results.append(result)
        self.total_tests += 1
        if result.triage_correct:
            self.triage_correct += 1
        if result.executed:
            self.executed += 1
        if result.gatekeeper_passed:
            self.passed_gatekeeper += 1

        # Update average score
        scores = [r.final_score for r in self.results if r.executed]
        if scores:
            self.avg_score = sum(scores) / len(scores)


class WorkflowTestRunner:
    """Runs test projects through the workflow system."""

    def __init__(self, backend: str = "auto", dry_run: bool = False):
        """
        Initialize the workflow test runner.

        Args:
            backend: Model selection mode
                - "auto": Use DEFAULT_ROLE_MODELS for each role (recommended)
                - "claude": Force all agents to use Claude
                - "gemini": Force all agents to use Gemini
            dry_run: If True, only test triage routing (no execution)
        """
        self.dry_run = dry_run
        self.backend = backend
        self.role_models = {}  # Track which model each role uses

        # Create triage agent - respects backend override
        if backend == "claude":
            triage_runner = ClaudeRunner()
            self.role_models["triage"] = "claude"
        elif backend == "gemini":
            triage_runner = GeminiRunner(backend="gemini")
            self.role_models["triage"] = "gemini"
        elif backend == "groq":
            triage_runner = GroqRunner()
            self.role_models["triage"] = "groq"
        else:
            triage_runner = get_runner_for_role("triage")
            self.role_models["triage"] = get_model_for_role("triage")
        self.triage = create_triage(triage_runner)

        # Role to AgentRole enum mapping
        role_enum_map = {
            "architect": AgentRole.ARCHITECT,
            "executor": AgentRole.EXECUTOR,
            "critic": AgentRole.CRITIC,
            "planner": AgentRole.PLANNER,
            "debugger": AgentRole.DEBUGGER,
            "researcher": AgentRole.RESEARCHER,
            "verifier": AgentRole.VERIFIER,
            "tester": AgentRole.TESTER,
            "documenter": AgentRole.DOCUMENTER,
        }

        # Create agents for each role with appropriate runners
        self.agents = {}
        for role_name, role_enum in role_enum_map.items():
            if backend == "auto":
                # Use config-based model selection
                runner = get_runner_for_role(role_name)
                model_name = get_model_for_role(role_name)
            elif backend == "claude":
                runner = ClaudeRunner()
                model_name = "claude"
            elif backend == "gemini":
                runner = GeminiRunner(backend="gemini")  # Use CLI, not API
                model_name = "gemini"
            elif backend == "groq":
                runner = GroqRunner()
                model_name = "groq"
            else:
                runner = ClaudeRunner()
                model_name = "claude"

            self.agents[role_name] = create_agent(role_name, runner, role_enum)
            self.role_models[role_name] = model_name

        # Scoring components - Judge uses Claude for quality evaluation
        judge_runner = get_runner_for_role("judge")
        self.gatekeeper = Gatekeeper()
        self.judge = Judge(judge_runner)
        self.score_store = ScoreStore()
        self.role_models["judge"] = get_model_for_role("judge")

        # Enable response scoring if not already
        config.enable("response_scoring")

        # Print model assignments
        print("\n📦 Model Assignments:")
        for role, model in sorted(self.role_models.items()):
            print(f"  {role:12} -> {model}")
        print()

    def run_triage(self, task: str) -> Dict[str, Any]:
        """Run triage to determine workflow."""
        print(f"  Triaging task...", end="", flush=True)

        response = self.triage.invoke(task)

        try:
            # Parse JSON response
            decision = json.loads(response.content)
            print(f" -> {decision.get('workflow', 'UNKNOWN')}")
            return decision
        except json.JSONDecodeError:
            # Try to extract workflow from text
            content = response.content.lower()
            for workflow in WORKFLOWS.keys():
                if workflow in content:
                    print(f" -> {workflow} (extracted)")
                    return {
                        "workflow": workflow,
                        "roles": WORKFLOWS[workflow]["roles"],
                        "reasoning": "Extracted from text response",
                        "answer": None
                    }
            print(" -> UNKNOWN (parse failed)")
            return {
                "workflow": "unknown",
                "roles": [],
                "reasoning": f"Failed to parse: {response.content[:100]}",
                "answer": None
            }

    def execute_workflow(self, task: str, decision: Dict[str, Any]) -> str:
        """Execute the selected workflow."""
        workflow_name = decision.get("workflow", "simple").lower()

        # Handle QUICK - direct answer from triage
        if workflow_name == "quick":
            answer = decision.get("answer")
            if answer:
                return answer
            # If no answer, use executor as fallback
            return self.agents["executor"].invoke(task).content

        # Get roles for workflow
        roles = decision.get("roles", WORKFLOWS.get(workflow_name, {}).get("roles", ["executor"]))

        if not roles:
            roles = ["executor"]

        # Execute each role in sequence
        context = task
        final_response = ""

        for role_name in roles:
            agent = self.agents.get(role_name)
            if not agent:
                print(f"    Warning: No agent for role '{role_name}', skipping")
                continue

            model = self.role_models.get(role_name, "unknown")
            print(f"    Running {role_name} ({model})...", end="", flush=True)
            start = time.time()

            response = agent.invoke(
                f"Task: {task}\n\nPrevious context:\n{context[:2000]}",
                context=context[:2000] if len(context) > 100 else None
            )

            elapsed = time.time() - start
            print(f" done ({elapsed:.1f}s)")

            context = response.content
            final_response = response.content

        return final_response

    def score_response(self, task: str, response: str, project: dict) -> Dict[str, float]:
        """Score a response using Gatekeeper and Judge."""
        scores = {
            "gatekeeper": 0.0,
            "gatekeeper_passed": False,
            "judge": 0.0,
            "final": 0.0,
        }

        # Build requirements based on project complexity
        workflow = project.get("expected_workflow", "simple")
        expected_keywords = project.get("expected_answer_contains", [])

        # Adjust requirements based on workflow complexity
        if workflow == "quick":
            min_length = 1  # Very lenient for quick answers
        elif workflow == "simple":
            min_length = 20
        elif workflow == "standard":
            min_length = 100
        else:
            min_length = 200

        requirements = {
            "min_length": min_length,
            "must_contain": expected_keywords,
        }

        gate_result = self.gatekeeper.validate(response, requirements)
        scores["gatekeeper"] = gate_result.score if hasattr(gate_result, 'score') else (1.0 if gate_result.passed else 0.0)
        scores["gatekeeper_passed"] = gate_result.passed

        # Judge evaluation (only if gatekeeper passed)
        if gate_result.passed:
            print(f"    Running Judge evaluation...", end="", flush=True)

            # Build a benchmark dict for the Judge
            benchmark = {
                "prompt": task,
                "rubric": {
                    "correctness": "Is the response correct and accurate?",
                    "completeness": "Does it fully address the task?",
                    "quality": "Is the code/answer well-structured?",
                },
                "requirements": requirements,
            }

            try:
                judge_result = self.judge.evaluate(
                    response=response,
                    role=workflow,  # Use workflow as role
                    benchmark=benchmark
                )
                scores["judge"] = judge_result.overall_score if hasattr(judge_result, 'overall_score') else 7.0
                print(f" {scores['judge']}/10")
            except Exception as e:
                print(f" error: {e}")
                scores["judge"] = 5.0  # Default if judge fails

        # Calculate final score (weighted average)
        gatekeeper_weight = config.get("gatekeeper_weight", 0.3)
        judge_weight = config.get("judge_weight", 0.7)

        if scores["gatekeeper_passed"]:
            scores["final"] = (scores["gatekeeper"] * gatekeeper_weight * 10 +
                             scores["judge"] * judge_weight)
        else:
            scores["final"] = scores["gatekeeper"] * 3  # Penalty for failing gatekeeper

        return scores

    def run_test(self, project: dict) -> TestResult:
        """Run a single test project."""
        print(f"\n{'='*60}")
        print(f"Testing: [{project['id']}] {project['name']}")
        print(f"Expected workflow: {project['expected_workflow']}")
        print(f"Task: {project['task'][:80]}...")
        print(f"{'='*60}")

        result = TestResult(
            project_id=project["id"],
            project_name=project["name"],
            task=project["task"],
            triage_workflow="",
            expected_workflow=project["expected_workflow"],
            triage_correct=False,
            triage_reasoning="",
            backend=self.backend,
        )

        try:
            # Step 1: Triage
            decision = self.run_triage(project["task"])
            result.triage_workflow = decision.get("workflow", "unknown").lower()
            result.triage_reasoning = decision.get("reasoning", "")
            result.triage_correct = (result.triage_workflow == project["expected_workflow"])

            print(f"  Triage result: {result.triage_workflow} "
                  f"({'CORRECT' if result.triage_correct else 'WRONG - expected ' + project['expected_workflow']})")

            # Step 2: Execute (unless dry run)
            if not self.dry_run:
                print(f"  Executing workflow...")
                start = time.time()
                result.response = self.execute_workflow(project["task"], decision)
                result.execution_time = time.time() - start
                result.executed = True

                print(f"  Response ({len(result.response)} chars, {result.execution_time:.1f}s):")
                print(f"    {result.response[:200].replace(chr(10), ' ')}...")

                # Step 3: Score
                print(f"  Scoring response...")
                scores = self.score_response(project["task"], result.response, project)
                result.gatekeeper_score = scores["gatekeeper"]
                result.gatekeeper_passed = scores["gatekeeper_passed"]
                result.judge_score = scores["judge"]
                result.final_score = scores["final"]

                print(f"  Final score: {result.final_score:.1f}/10 "
                      f"(Gate: {'PASS' if result.gatekeeper_passed else 'FAIL'}, "
                      f"Judge: {result.judge_score:.1f})")

        except Exception as e:
            result.error = str(e)
            print(f"  ERROR: {e}")

        return result

    def run_all(self, projects: List[dict] = None) -> TestSummary:
        """Run all test projects."""
        projects = projects or TEST_PROJECTS
        summary = TestSummary()

        backend_desc = "auto (role-based selection)" if self.backend == "auto" else self.backend
        print(f"\n{'#'*60}")
        print(f"# Workflow Test Runner")
        print(f"# Backend: {backend_desc}")
        print(f"# Mode: {'DRY RUN (triage only)' if self.dry_run else 'FULL (execute + score)'}")
        print(f"# Projects: {len(projects)}")
        print(f"{'#'*60}")

        for project in projects:
            result = self.run_test(project)
            summary.add_result(result)

        return summary


def save_results(summary: TestSummary, output_dir: Path, backend: str = "auto"):
    """Save test results to files with dated filenames."""
    output_dir.mkdir(parents=True, exist_ok=True)

    date_str = datetime.now().strftime("%Y-%m-%d")

    # Save full results as JSON
    results_file = output_dir / f"{date_str}_workflow_results.json"
    with open(results_file, "w") as f:
        json.dump({
            "meta": {
                "date": date_str,
                "backend": backend,
                "timestamp": datetime.now().isoformat(),
            },
            "summary": {
                "total_tests": summary.total_tests,
                "triage_correct": summary.triage_correct,
                "triage_accuracy": summary.triage_correct / summary.total_tests if summary.total_tests else 0,
                "executed": summary.executed,
                "passed_gatekeeper": summary.passed_gatekeeper,
                "avg_score": round(summary.avg_score, 2),
            },
            "results": [asdict(r) for r in summary.results]
        }, f, indent=2)

    print(f"\nResults saved to: {results_file}")

    # Generate markdown summary
    summary_file = output_dir / f"{date_str}_workflow_summary.md"
    with open(summary_file, "w") as f:
        f.write("# Workflow Benchmark Summary\n\n")
        f.write(f"**Date**: {datetime.now().strftime('%Y-%m-%d %H:%M')}\n\n")

        f.write("## Overview\n\n")
        f.write(f"| Metric | Value |\n")
        f.write(f"|--------|-------|\n")
        f.write(f"| Total Tests | {summary.total_tests} |\n")
        f.write(f"| Triage Accuracy | {summary.triage_correct}/{summary.total_tests} ({summary.triage_correct/summary.total_tests*100:.0f}%) |\n")
        f.write(f"| Executed | {summary.executed} |\n")
        f.write(f"| Passed Gatekeeper | {summary.passed_gatekeeper} |\n")
        f.write(f"| Average Score | {summary.avg_score:.1f}/10 |\n\n")

        f.write("## Results by Workflow\n\n")

        for workflow in ["quick", "simple", "standard", "full", "debug"]:
            workflow_results = [r for r in summary.results if r.expected_workflow == workflow]
            if not workflow_results:
                continue

            f.write(f"### {workflow.upper()}\n\n")
            f.write("| Project | Triage | Score | Status |\n")
            f.write("|---------|--------|-------|--------|\n")

            for r in workflow_results:
                triage_status = "CORRECT" if r.triage_correct else f"WRONG ({r.triage_workflow})"
                score = f"{r.final_score:.1f}" if r.executed else "N/A"
                status = "PASS" if r.gatekeeper_passed else ("FAIL" if r.executed else "NOT RUN")
                f.write(f"| {r.project_name} | {triage_status} | {score} | {status} |\n")

            f.write("\n")

        f.write("## Detailed Results\n\n")
        for r in summary.results:
            f.write(f"### [{r.project_id}] {r.project_name}\n\n")
            f.write(f"- **Task**: {r.task[:100]}...\n")
            f.write(f"- **Expected Workflow**: {r.expected_workflow}\n")
            f.write(f"- **Triage Result**: {r.triage_workflow} ({'CORRECT' if r.triage_correct else 'WRONG'})\n")
            f.write(f"- **Reasoning**: {r.triage_reasoning[:200]}...\n" if r.triage_reasoning else "")
            if r.executed:
                f.write(f"- **Execution Time**: {r.execution_time:.1f}s\n")
                f.write(f"- **Gatekeeper**: {'PASS' if r.gatekeeper_passed else 'FAIL'} ({r.gatekeeper_score:.1f})\n")
                f.write(f"- **Judge Score**: {r.judge_score:.1f}/10\n")
                f.write(f"- **Final Score**: {r.final_score:.1f}/10\n")
            if r.error:
                f.write(f"- **Error**: {r.error}\n")
            f.write("\n")

    print(f"Summary saved to: {summary_file}")


def print_summary(summary: TestSummary):
    """Print summary to console."""
    print(f"\n{'='*60}")
    print("SUMMARY")
    print(f"{'='*60}")
    print(f"Total Tests:       {summary.total_tests}")
    print(f"Triage Correct:    {summary.triage_correct}/{summary.total_tests} "
          f"({summary.triage_correct/summary.total_tests*100:.0f}%)")
    print(f"Executed:          {summary.executed}")
    print(f"Passed Gatekeeper: {summary.passed_gatekeeper}")
    print(f"Average Score:     {summary.avg_score:.1f}/10")
    print(f"{'='*60}")


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Run workflow tests")
    parser.add_argument("project_id", nargs="?", help="Specific project ID to run")
    parser.add_argument("--workflow", help="Run all tests for a specific workflow")
    parser.add_argument("--dry-run", action="store_true", help="Only test triage routing")
    parser.add_argument("--backend", default="auto",
                        help="Backend: auto (role-based selection), claude, gemini, groq")
    parser.add_argument("--output", default=str(Path(__file__).parent), help="Output directory")

    args = parser.parse_args()

    # Select projects
    if args.project_id:
        project = get_project_by_id(args.project_id)
        if not project:
            print(f"Unknown project: {args.project_id}")
            sys.exit(1)
        projects = [project]
    elif args.workflow:
        projects = get_projects_by_workflow(args.workflow)
        if not projects:
            print(f"No projects for workflow: {args.workflow}")
            sys.exit(1)
    else:
        projects = TEST_PROJECTS

    # Run tests
    runner = WorkflowTestRunner(backend=args.backend, dry_run=args.dry_run)
    summary = runner.run_all(projects)

    # Output
    print_summary(summary)
    save_results(summary, Path(args.output), args.backend)


if __name__ == "__main__":
    main()
