"""
Benchmarking Module - Compare responses across multiple backends

Enables running the same prompt through different backends (Claude, Gemini, Ollama, etc.)
and comparing results for quality, speed, and cost.
"""
import time
import json
from dataclasses import dataclass, field
from typing import List, Dict, Any, Optional, Callable
from pathlib import Path
from datetime import datetime
from concurrent.futures import ThreadPoolExecutor, as_completed

# Handle imports for both module and standalone usage
try:
    from .runners.base import CLIRunner, RunnerResult
except ImportError:
    from runners.base import CLIRunner, RunnerResult


@dataclass
class BenchmarkResult:
    """Result from a single backend run."""
    backend: str
    model: str
    response: str
    execution_time: float
    success: bool
    error: Optional[str] = None
    tokens: Optional[int] = None
    metadata: Dict[str, Any] = field(default_factory=dict)


@dataclass
class BenchmarkComparison:
    """Comparison of results across multiple backends."""
    prompt: str
    results: List[BenchmarkResult]
    timestamp: datetime = field(default_factory=datetime.now)
    winner: Optional[str] = None  # Best backend based on criteria
    scores: Dict[str, float] = field(default_factory=dict)

    def fastest(self) -> Optional[BenchmarkResult]:
        """Get the fastest successful result."""
        successful = [r for r in self.results if r.success]
        if not successful:
            return None
        return min(successful, key=lambda r: r.execution_time)

    def by_backend(self, backend: str) -> Optional[BenchmarkResult]:
        """Get result for a specific backend."""
        for r in self.results:
            if r.backend == backend:
                return r
        return None

    def summary(self) -> str:
        """Generate a summary of the comparison."""
        lines = [
            f"Benchmark: {self.prompt[:50]}...",
            f"Timestamp: {self.timestamp.isoformat()}",
            "",
            "Results:"
        ]

        for r in sorted(self.results, key=lambda x: x.execution_time):
            status = "✓" if r.success else "✗"
            lines.append(
                f"  {status} {r.backend} ({r.model}): {r.execution_time:.2f}s"
            )
            if r.error:
                lines.append(f"      Error: {r.error}")

        fastest = self.fastest()
        if fastest:
            lines.append(f"\nFastest: {fastest.backend} ({fastest.execution_time:.2f}s)")

        return "\n".join(lines)

    def to_dict(self) -> dict:
        return {
            "prompt": self.prompt,
            "timestamp": self.timestamp.isoformat(),
            "results": [
                {
                    "backend": r.backend,
                    "model": r.model,
                    "execution_time": r.execution_time,
                    "success": r.success,
                    "response_length": len(r.response),
                    "error": r.error
                }
                for r in self.results
            ],
            "winner": self.winner,
            "scores": self.scores
        }


class Benchmarker:
    """
    Run benchmarks across multiple backends.

    Usage:
        benchmarker = Benchmarker()
        benchmarker.add_runner(ClaudeRunner())
        benchmarker.add_runner(GeminiRunner())

        comparison = benchmarker.run("Explain quantum computing")
        print(comparison.summary())
    """

    def __init__(
        self,
        parallel: bool = True,
        max_workers: int = 4,
        debug: bool = False
    ):
        self.runners: List[CLIRunner] = []
        self.parallel = parallel
        self.max_workers = max_workers
        self.debug = debug
        self._history: List[BenchmarkComparison] = []

    def add_runner(self, runner: CLIRunner) -> "Benchmarker":
        """Add a runner to benchmark."""
        if runner.is_available():
            self.runners.append(runner)
            if self.debug:
                print(f"Added runner: {runner.name}")
        else:
            if self.debug:
                print(f"Skipped unavailable runner: {runner.name}")
        return self

    def run(
        self,
        prompt: str,
        scorer: Optional[Callable[[str], float]] = None,
        **kwargs
    ) -> BenchmarkComparison:
        """
        Run the prompt through all configured backends.

        Args:
            prompt: The prompt to benchmark
            scorer: Optional function to score responses (higher = better)
            **kwargs: Additional arguments passed to runners

        Returns:
            BenchmarkComparison with all results
        """
        if not self.runners:
            raise ValueError("No runners configured. Call add_runner() first.")

        results = []

        if self.parallel:
            results = self._run_parallel(prompt, **kwargs)
        else:
            results = self._run_sequential(prompt, **kwargs)

        comparison = BenchmarkComparison(
            prompt=prompt,
            results=results
        )

        # Score responses if scorer provided
        if scorer:
            for result in results:
                if result.success:
                    try:
                        comparison.scores[result.backend] = scorer(result.response)
                    except Exception as e:
                        if self.debug:
                            print(f"Scorer error for {result.backend}: {e}")

            # Determine winner
            if comparison.scores:
                comparison.winner = max(comparison.scores, key=comparison.scores.get)

        self._history.append(comparison)
        return comparison

    def _run_sequential(self, prompt: str, **kwargs) -> List[BenchmarkResult]:
        """Run benchmarks sequentially."""
        results = []

        for runner in self.runners:
            if self.debug:
                print(f"Running {runner.name}...")

            start_time = time.time()
            result = runner.run(prompt, **kwargs)
            execution_time = time.time() - start_time

            results.append(BenchmarkResult(
                backend=runner.name,
                model=result.model,
                response=result.content,
                execution_time=execution_time,
                success=result.success,
                error=result.error,
                tokens=result.tokens_used,
                metadata=result.metadata
            ))

        return results

    def _run_parallel(self, prompt: str, **kwargs) -> List[BenchmarkResult]:
        """Run benchmarks in parallel."""
        results = []

        def run_single(runner: CLIRunner) -> BenchmarkResult:
            if self.debug:
                print(f"Starting {runner.name}...")

            start_time = time.time()
            result = runner.run(prompt, **kwargs)
            execution_time = time.time() - start_time

            if self.debug:
                print(f"Completed {runner.name} in {execution_time:.2f}s")

            return BenchmarkResult(
                backend=runner.name,
                model=result.model,
                response=result.content,
                execution_time=execution_time,
                success=result.success,
                error=result.error,
                tokens=result.tokens_used,
                metadata=result.metadata
            )

        with ThreadPoolExecutor(max_workers=self.max_workers) as executor:
            futures = {executor.submit(run_single, r): r for r in self.runners}

            for future in as_completed(futures):
                try:
                    result = future.result()
                    results.append(result)
                except Exception as e:
                    runner = futures[future]
                    results.append(BenchmarkResult(
                        backend=runner.name,
                        model="",
                        response="",
                        execution_time=0,
                        success=False,
                        error=str(e)
                    ))

        return results

    def run_suite(
        self,
        prompts: List[str],
        save_path: Optional[Path] = None
    ) -> List[BenchmarkComparison]:
        """
        Run a suite of benchmark prompts.

        Args:
            prompts: List of prompts to benchmark
            save_path: Optional path to save results as JSON

        Returns:
            List of BenchmarkComparison results
        """
        results = []

        for i, prompt in enumerate(prompts):
            if self.debug:
                print(f"\n[Suite] Running prompt {i+1}/{len(prompts)}")

            comparison = self.run(prompt)
            results.append(comparison)

        if save_path:
            self.save_results(results, save_path)

        return results

    def save_results(
        self,
        results: List[BenchmarkComparison],
        path: Path
    ) -> None:
        """Save benchmark results to JSON."""
        data = {
            "timestamp": datetime.now().isoformat(),
            "runners": [r.name for r in self.runners],
            "results": [r.to_dict() for r in results]
        }

        with open(path, 'w') as f:
            json.dump(data, f, indent=2)

    def history(self) -> List[BenchmarkComparison]:
        """Get all benchmark runs from this session."""
        return self._history

    def aggregate_stats(self) -> Dict[str, Dict[str, float]]:
        """
        Get aggregate statistics across all benchmark runs.

        Returns:
            Dict with stats per backend
        """
        stats = {}

        for backend in set(r.name for r in self.runners):
            backend_results = [
                result
                for comparison in self._history
                for result in comparison.results
                if result.backend == backend and result.success
            ]

            if backend_results:
                times = [r.execution_time for r in backend_results]
                stats[backend] = {
                    "count": len(backend_results),
                    "avg_time": sum(times) / len(times),
                    "min_time": min(times),
                    "max_time": max(times),
                    "success_rate": len(backend_results) / len([
                        r for c in self._history
                        for r in c.results
                        if r.backend == backend
                    ])
                }

        return stats


# Scoring functions for common use cases
def length_scorer(response: str) -> float:
    """Score by response length (longer = better for detailed tasks)."""
    return len(response)


def brevity_scorer(response: str) -> float:
    """Score by brevity (shorter = better for concise tasks)."""
    return 1.0 / (len(response) + 1)


def code_block_scorer(response: str) -> float:
    """Score by number of code blocks (good for coding tasks)."""
    return response.count("```")


def quality_indicators_scorer(response: str) -> float:
    """
    Score by presence of quality indicators.
    Looks for structure, examples, explanations.
    """
    score = 0.0

    # Headers indicate structure
    score += response.count("#") * 0.5

    # Code blocks
    score += response.count("```") * 2

    # Lists indicate organized thinking
    score += response.count("- ") * 0.3
    score += response.count("1. ") * 0.3

    # Examples
    if "example" in response.lower():
        score += 1

    return score
