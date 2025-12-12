"""
Tests for Model Evaluator - Meritocratic Model Selection System

Tests the complete evaluation pipeline:
- Gatekeeper (heuristic validation)
- Judge (LLM quality scoring)
- ScoreStore (persistence)
- ModelSelector (smart selection)
"""
import json
import sys
import pytest
import tempfile
from pathlib import Path
from datetime import datetime
from unittest.mock import MagicMock, patch

# Add parent to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from model_evaluator import (
    ROLE_BENCHMARKS,
    Gatekeeper,
    GatekeeperResult,
    Judge,
    JudgeResult,
    ScoreStore,
    ModelSelector,
    ModelEvaluator,
    EvaluationResult,
    list_roles,
    get_role_benchmark,
)
from runners.base import CLIRunner, RunnerResult


# =============================================================================
# Test Fixtures
# =============================================================================

@pytest.fixture
def mock_runner():
    """Create a mock CLIRunner."""
    runner = MagicMock(spec=CLIRunner)
    runner.name = "mock"
    runner.is_available.return_value = True
    return runner


@pytest.fixture
def temp_score_store(tmp_path):
    """Create a ScoreStore with temporary storage."""
    store_path = tmp_path / "test_scores.json"
    return ScoreStore(storage_path=store_path)


@pytest.fixture
def gatekeeper():
    """Create a Gatekeeper instance."""
    return Gatekeeper()


@pytest.fixture
def sample_good_executor_response():
    """Sample response that should pass executor requirements."""
    return '''```python
def validate_email(email: str) -> bool:
    """
    Validate an email address.

    Args:
        email: The email string to validate

    Returns:
        True if valid, False otherwise

    Examples:
        >>> validate_email("user@example.com")
        True
        >>> validate_email("invalid")
        False
    """
    import re

    if not email or not isinstance(email, str):
        return False

    # Basic email pattern
    pattern = r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'

    if not re.match(pattern, email):
        return False

    # Check for common edge cases
    if email.startswith('.') or email.endswith('.'):
        return False

    if '..' in email:
        return False

    return True
```'''


@pytest.fixture
def sample_bad_executor_response():
    """Sample response that should fail executor requirements."""
    return "Just check if there's an @ symbol in the email."


@pytest.fixture
def sample_good_critic_response():
    """Sample response that should pass critic requirements."""
    return '''## Code Review

### Issues Found

1. **Bug: Incorrect method name**
   - Line 4: `email.contains('@')` should be `'@' in email`
   - Python strings don't have a `.contains()` method
   - Severity: HIGH - This will cause an AttributeError

2. **Security Vulnerability: eval() usage**
   - Line 11: `eval(users)` is extremely dangerous
   - An attacker could execute arbitrary code
   - Severity: CRITICAL - Security vulnerability

3. **Resource Leak: Unclosed file**
   - Line 10: `open('users.txt').read()` doesn't close the file
   - Should use context manager: `with open(...) as f:`
   - Severity: MEDIUM - Resource leak

4. **Missing Error Handling**
   - No handling for missing keys in `user_data`
   - No handling for file not found
   - Severity: MEDIUM - Potential crashes

### Recommendations

- Fix `.contains()` to use `in` operator
- Replace `eval()` with `json.loads()` for safe parsing
- Use context manager for file operations
- Add try/except blocks for error handling

VERDICT: FAIL'''


# =============================================================================
# Role Benchmarks Tests
# =============================================================================

class TestRoleBenchmarks:
    """Tests for ROLE_BENCHMARKS configuration."""

    def test_all_roles_have_benchmarks(self):
        """All expected roles have benchmark definitions."""
        expected_roles = [
            "architect", "executor", "critic", "verifier",
            "researcher", "debugger", "tester", "documenter", "planner"
        ]
        for role in expected_roles:
            assert role in ROLE_BENCHMARKS, f"Missing benchmark for {role}"

    def test_benchmarks_have_required_fields(self):
        """Each benchmark has prompt, requirements, and rubric."""
        for role, benchmark in ROLE_BENCHMARKS.items():
            assert "prompt" in benchmark, f"{role} missing prompt"
            assert "requirements" in benchmark, f"{role} missing requirements"
            assert "rubric" in benchmark, f"{role} missing rubric"
            assert len(benchmark["prompt"]) > 100, f"{role} prompt too short"

    def test_requirements_have_min_length(self):
        """All benchmarks require minimum response length."""
        for role, benchmark in ROLE_BENCHMARKS.items():
            reqs = benchmark["requirements"]
            assert "min_length" in reqs, f"{role} missing min_length"
            assert reqs["min_length"] >= 200, f"{role} min_length too low"

    def test_rubric_has_items(self):
        """Each rubric has evaluation criteria."""
        for role, benchmark in ROLE_BENCHMARKS.items():
            rubric = benchmark["rubric"]
            assert len(rubric) >= 3, f"{role} rubric has too few items"

    def test_list_roles_function(self):
        """list_roles() returns all role names."""
        roles = list_roles()
        assert len(roles) == len(ROLE_BENCHMARKS)
        assert "executor" in roles
        assert "planner" in roles

    def test_get_role_benchmark_function(self):
        """get_role_benchmark() returns correct benchmark."""
        benchmark = get_role_benchmark("executor")
        assert "prompt" in benchmark
        assert "email" in benchmark["prompt"].lower()

    def test_get_role_benchmark_invalid(self):
        """get_role_benchmark() returns empty dict for invalid role."""
        benchmark = get_role_benchmark("nonexistent")
        assert benchmark == {}


# =============================================================================
# Gatekeeper Tests
# =============================================================================

class TestGatekeeper:
    """Tests for Gatekeeper heuristic validation."""

    def test_min_length_pass(self, gatekeeper):
        """Passes when response meets minimum length."""
        result = gatekeeper.validate(
            "x" * 500,
            {"min_length": 500}
        )
        assert result.checks.get("min_length") is True

    def test_min_length_fail(self, gatekeeper):
        """Fails when response is too short."""
        result = gatekeeper.validate(
            "x" * 100,
            {"min_length": 500}
        )
        assert result.checks.get("min_length") is False
        assert "too short" in result.failures[0].lower()

    def test_must_contain_pass(self, gatekeeper):
        """Passes when all required terms present."""
        result = gatekeeper.validate(
            "This has def and return and a docstring",
            {"must_contain": ["def", "return"]}
        )
        assert result.checks.get("contains_def") is True
        assert result.checks.get("contains_return") is True

    def test_must_contain_fail(self, gatekeeper):
        """Fails when required term missing."""
        result = gatekeeper.validate(
            "This only has def",
            {"must_contain": ["def", "return"]}
        )
        assert result.checks.get("contains_return") is False
        assert any("return" in f for f in result.failures)

    def test_must_contain_case_insensitive(self, gatekeeper):
        """must_contain is case insensitive."""
        result = gatekeeper.validate(
            "This has DEF and RETURN",
            {"must_contain": ["def", "return"]}
        )
        assert result.checks.get("contains_def") is True
        assert result.checks.get("contains_return") is True

    def test_code_blocks_pass(self, gatekeeper):
        """Passes when enough code blocks present."""
        response = "Here's code:\n```python\nprint('hi')\n```\nDone."
        result = gatekeeper.validate(
            response,
            {"code_blocks_min": 1}
        )
        assert result.checks.get("code_blocks") is True

    def test_code_blocks_fail(self, gatekeeper):
        """Fails when not enough code blocks."""
        result = gatekeeper.validate(
            "No code blocks here",
            {"code_blocks_min": 1}
        )
        assert result.checks.get("code_blocks") is False

    def test_structure_with_headers(self, gatekeeper):
        """Passes structure check with markdown headers."""
        result = gatekeeper.validate(
            "# Header\nSome content\n## Subheader\nMore content",
            {"must_have_structure": True}
        )
        assert result.checks.get("has_structure") is True

    def test_structure_with_lists(self, gatekeeper):
        """Passes structure check with lists."""
        result = gatekeeper.validate(
            "Items:\n- First\n- Second\n- Third",
            {"must_have_structure": True}
        )
        assert result.checks.get("has_structure") is True

    def test_structure_fail(self, gatekeeper):
        """Fails structure check without headers or lists."""
        result = gatekeeper.validate(
            "Just plain text without any structure at all.",
            {"must_have_structure": True}
        )
        assert result.checks.get("has_structure") is False

    def test_docstring_check(self, gatekeeper):
        """Checks for Python docstrings."""
        result = gatekeeper.validate(
            '"""This is a docstring."""',
            {"has_docstring": True}
        )
        assert result.checks.get("has_docstring") is True

    def test_verdict_check(self, gatekeeper):
        """Checks for VERDICT declaration."""
        result = gatekeeper.validate(
            "Review complete.\nVERDICT: PASS",
            {"must_contain": ["VERDICT"]}
        )
        assert result.checks.get("contains_VERDICT") is True

    def test_test_count_check(self, gatekeeper):
        """Counts test functions."""
        response = """
def test_add():
    assert add(1, 2) == 3

def test_subtract():
    assert subtract(5, 3) == 2

def test_multiply():
    assert multiply(2, 3) == 6
"""
        result = gatekeeper.validate(response, {"test_count_min": 3})
        assert result.checks.get("test_count") is True

    def test_clarifying_questions_check(self, gatekeeper):
        """Checks for clarifying questions."""
        result = gatekeeper.validate(
            "What database? What auth method? How many users? Any deadline?",
            {"asks_clarifying_questions": True}
        )
        assert result.checks.get("asks_questions") is True

    def test_overall_pass(self, gatekeeper, sample_good_executor_response):
        """Good executor response passes gatekeeper."""
        requirements = ROLE_BENCHMARKS["executor"]["requirements"]
        result = gatekeeper.validate(sample_good_executor_response, requirements)
        assert result.passed is True
        assert result.score >= 0.7

    def test_overall_fail(self, gatekeeper, sample_bad_executor_response):
        """Bad executor response fails gatekeeper."""
        requirements = ROLE_BENCHMARKS["executor"]["requirements"]
        result = gatekeeper.validate(sample_bad_executor_response, requirements)
        assert result.passed is False
        assert result.score < 0.7

    def test_critic_response_passes(self, gatekeeper, sample_good_critic_response):
        """Good critic response passes gatekeeper."""
        requirements = ROLE_BENCHMARKS["critic"]["requirements"]
        result = gatekeeper.validate(sample_good_critic_response, requirements)
        assert result.passed is True

    def test_score_calculation(self, gatekeeper):
        """Score is percentage of checks passed."""
        result = gatekeeper.validate(
            "x" * 500 + " def return",  # Passes length and both contains
            {"min_length": 500, "must_contain": ["def", "return"]}
        )
        assert result.score == 1.0  # All 3 checks pass

    def test_empty_requirements(self, gatekeeper):
        """Empty requirements result in pass."""
        result = gatekeeper.validate("anything", {})
        assert result.passed is True
        assert result.score == 1.0


# =============================================================================
# Judge Tests
# =============================================================================

class TestJudge:
    """Tests for Judge LLM evaluation."""

    def test_judge_creation(self, mock_runner):
        """Judge can be created with a runner."""
        judge = Judge(mock_runner)
        assert judge.runner == mock_runner

    def test_parse_judge_response_scores(self, mock_runner):
        """Parses scores from judge response."""
        judge = Judge(mock_runner)
        content = """
RUBRIC_SCORES:
- correctness: 8/10 - Good implementation
- edge_cases: 7/10 - Most cases covered

OVERALL_SCORE: 7.5/10

STRENGTHS:
- Clean code
- Good naming

WEAKNESSES:
- Missing some edge cases
"""
        rubric = {"correctness": "", "edge_cases": ""}
        result = judge._parse_judge_response(content, rubric)

        assert result.scores["correctness"] == 8.0
        assert result.scores["edge_cases"] == 7.0
        assert result.overall_score == 7.5
        assert len(result.strengths) == 2
        assert len(result.weaknesses) == 1

    def test_parse_judge_response_defaults(self, mock_runner):
        """Uses defaults when parsing fails."""
        judge = Judge(mock_runner)
        result = judge._parse_judge_response("Invalid response", {"quality": ""})

        assert result.scores["quality"] == 5.0  # Default
        assert 1.0 <= result.overall_score <= 10.0

    def test_evaluate_with_mock(self, mock_runner):
        """Evaluate calls runner and parses response."""
        mock_runner.run.return_value = RunnerResult(
            content="""
RUBRIC_SCORES:
- completeness: 9/10 - Very thorough

OVERALL_SCORE: 9.0/10

STRENGTHS:
- Excellent coverage

WEAKNESSES:
- Minor formatting
""",
            success=True,
            backend="mock",
            model="test-model"
        )

        judge = Judge(mock_runner)
        benchmark = {"prompt": "Test", "rubric": {"completeness": "Is it complete?"}}
        result = judge.evaluate("Sample response", "tester", benchmark)

        assert mock_runner.run.called
        assert result.overall_score == 9.0

    def test_evaluate_runner_failure(self, mock_runner):
        """Returns default scores when runner fails."""
        mock_runner.run.return_value = RunnerResult(
            content="",
            success=False,
            error="Connection failed",
            backend="mock"
        )

        judge = Judge(mock_runner)
        result = judge.evaluate("Sample", "tester", {"rubric": {"quality": ""}})

        assert result.overall_score == 5.0
        assert "failed" in result.feedback.lower()


# =============================================================================
# ScoreStore Tests
# =============================================================================

class TestScoreStore:
    """Tests for ScoreStore persistence."""

    def test_store_creation(self, temp_score_store):
        """ScoreStore creates storage file."""
        assert temp_score_store.storage_path.parent.exists()

    def test_record_and_retrieve(self, temp_score_store):
        """Can record and retrieve scores."""
        result = EvaluationResult(
            role="executor",
            model="test-model",
            backend="test",
            gatekeeper=GatekeeperResult(True, {}, [], 0.9),
            judge=JudgeResult({"quality": 8.0}, 8.0, "", [], []),
            response="",
            execution_time=1.5
        )
        temp_score_store.record(result)

        score = temp_score_store.get_score("test", "test-model", "executor")
        assert score is not None
        assert score > 0

    def test_get_best_for_role(self, temp_score_store):
        """Gets best model for a role."""
        # Record two results with different scores
        result1 = EvaluationResult(
            role="executor",
            model="model-a",
            backend="backend-a",
            gatekeeper=GatekeeperResult(True, {}, [], 0.8),
            judge=JudgeResult({}, 7.0, "", [], []),
            response="",
            execution_time=1.0
        )
        result2 = EvaluationResult(
            role="executor",
            model="model-b",
            backend="backend-b",
            gatekeeper=GatekeeperResult(True, {}, [], 0.9),
            judge=JudgeResult({}, 9.0, "", [], []),
            response="",
            execution_time=2.0
        )

        temp_score_store.record(result1)
        temp_score_store.record(result2)

        best = temp_score_store.get_best_for_role("executor")
        assert best is not None
        assert best[0] == "backend-b"  # Higher score
        assert best[1] == "model-b"

    def test_get_leaderboard(self, temp_score_store):
        """Gets ranked leaderboard for role."""
        for i, score in enumerate([6.0, 9.0, 7.5]):
            result = EvaluationResult(
                role="tester",
                model=f"model-{i}",
                backend=f"backend-{i}",
                gatekeeper=GatekeeperResult(True, {}, [], 0.8),
                judge=JudgeResult({}, score, "", [], []),
                response="",
                execution_time=1.0
            )
            temp_score_store.record(result)

        leaderboard = temp_score_store.get_leaderboard("tester")
        assert len(leaderboard) == 3
        # Should be sorted descending by score
        assert leaderboard[0]["model"] == "model-1"  # 9.0
        assert leaderboard[1]["model"] == "model-2"  # 7.5
        assert leaderboard[2]["model"] == "model-0"  # 6.0

    def test_persistence(self, tmp_path):
        """Scores persist across ScoreStore instances."""
        store_path = tmp_path / "persist_test.json"

        # First instance - record score
        store1 = ScoreStore(storage_path=store_path)
        result = EvaluationResult(
            role="architect",
            model="persist-model",
            backend="persist-backend",
            gatekeeper=GatekeeperResult(True, {}, [], 0.9),
            judge=JudgeResult({}, 8.5, "", [], []),
            response="",
            execution_time=1.0
        )
        store1.record(result)

        # Second instance - should load saved data
        store2 = ScoreStore(storage_path=store_path)
        score = store2.get_score("persist-backend", "persist-model", "architect")
        assert score is not None

    def test_no_data_returns_none(self, temp_score_store):
        """Returns None when no data available."""
        assert temp_score_store.get_score("x", "y", "z") is None
        assert temp_score_store.get_best_for_role("nonexistent") is None


# =============================================================================
# ModelSelector Tests
# =============================================================================

class TestModelSelector:
    """Tests for ModelSelector smart selection."""

    def test_select_with_config_override(self, temp_score_store):
        """Config override takes precedence."""
        selector = ModelSelector(
            temp_score_store,
            config_overrides={"architect": "custom:custom-model"}
        )
        backend, model = selector.select("architect")
        assert backend == "custom"
        assert model == "custom-model"

    def test_select_best_from_scores(self, temp_score_store):
        """Selects best model from benchmark scores."""
        # Record a good score
        result = EvaluationResult(
            role="executor",
            model="best-model",
            backend="best-backend",
            gatekeeper=GatekeeperResult(True, {}, [], 1.0),
            judge=JudgeResult({}, 9.0, "", [], []),
            response="",
            execution_time=1.0
        )
        temp_score_store.record(result)

        selector = ModelSelector(temp_score_store)
        backend, model = selector.select("executor")
        assert backend == "best-backend"
        assert model == "best-model"

    def test_select_falls_back_to_default(self, temp_score_store):
        """Falls back to default when no scores available."""
        selector = ModelSelector(
            temp_score_store,
            default_backend="fallback",
            default_model="fallback-model"
        )
        backend, model = selector.select("researcher")
        assert backend == "fallback"
        assert model == "fallback-model"

    def test_select_respects_min_score(self, temp_score_store):
        """Doesn't select model below minimum score."""
        # Record a low score
        result = EvaluationResult(
            role="critic",
            model="low-model",
            backend="low-backend",
            gatekeeper=GatekeeperResult(True, {}, [], 0.5),
            judge=JudgeResult({}, 4.0, "", [], []),
            response="",
            execution_time=1.0
        )
        temp_score_store.record(result)

        selector = ModelSelector(
            temp_score_store,
            default_backend="default",
            default_model="default-model"
        )
        # Min score is 6.0 by default
        backend, model = selector.select("critic")
        assert backend == "default"  # Falls back because score < 6.0

    def test_get_selection_reason_override(self, temp_score_store):
        """Explains config override selection."""
        selector = ModelSelector(
            temp_score_store,
            config_overrides={"architect": "claude:opus"}
        )
        reason = selector.get_selection_reason("architect")
        assert "override" in reason.lower()

    def test_get_selection_reason_benchmark(self, temp_score_store):
        """Explains benchmark-based selection."""
        result = EvaluationResult(
            role="tester",
            model="tested-model",
            backend="tested-backend",
            gatekeeper=GatekeeperResult(True, {}, [], 1.0),
            judge=JudgeResult({}, 8.0, "", [], []),
            response="",
            execution_time=1.0
        )
        temp_score_store.record(result)

        selector = ModelSelector(temp_score_store)
        reason = selector.get_selection_reason("tester")
        assert "benchmark" in reason.lower()


# =============================================================================
# EvaluationResult Tests
# =============================================================================

class TestEvaluationResult:
    """Tests for EvaluationResult dataclass."""

    def test_passed_when_both_pass(self):
        """Passed is True when gatekeeper passes and judge >= 6."""
        result = EvaluationResult(
            role="test",
            model="test",
            backend="test",
            gatekeeper=GatekeeperResult(True, {}, [], 0.9),
            judge=JudgeResult({}, 7.0, "", [], []),
            response="",
            execution_time=1.0
        )
        assert result.passed is True

    def test_failed_when_gatekeeper_fails(self):
        """Passed is False when gatekeeper fails."""
        result = EvaluationResult(
            role="test",
            model="test",
            backend="test",
            gatekeeper=GatekeeperResult(False, {}, ["fail"], 0.3),
            judge=JudgeResult({}, 9.0, "", [], []),
            response="",
            execution_time=1.0
        )
        assert result.passed is False

    def test_failed_when_judge_low(self):
        """Passed is False when judge score < 6."""
        result = EvaluationResult(
            role="test",
            model="test",
            backend="test",
            gatekeeper=GatekeeperResult(True, {}, [], 0.9),
            judge=JudgeResult({}, 5.0, "", [], []),
            response="",
            execution_time=1.0
        )
        assert result.passed is False

    def test_final_score_calculation(self):
        """Final score is 30% gatekeeper + 70% judge."""
        result = EvaluationResult(
            role="test",
            model="test",
            backend="test",
            gatekeeper=GatekeeperResult(True, {}, [], 1.0),  # 10/10 scaled
            judge=JudgeResult({}, 10.0, "", [], []),
            response="",
            execution_time=1.0
        )
        # (10 * 0.3) + (10 * 0.7) = 3 + 7 = 10
        assert result.final_score == 10.0

    def test_to_dict(self):
        """to_dict serializes correctly."""
        result = EvaluationResult(
            role="executor",
            model="test-model",
            backend="test-backend",
            gatekeeper=GatekeeperResult(True, {}, [], 0.9),
            judge=JudgeResult({}, 8.0, "", [], []),
            response="",
            execution_time=1.5
        )
        data = result.to_dict()

        assert data["role"] == "executor"
        assert data["model"] == "test-model"
        assert data["backend"] == "test-backend"
        assert data["passed"] is True
        assert "final_score" in data
        assert "timestamp" in data


# =============================================================================
# ModelEvaluator Integration Tests
# =============================================================================

class TestModelEvaluatorIntegration:
    """Integration tests for ModelEvaluator."""

    def test_evaluator_creation(self, mock_runner, temp_score_store):
        """Evaluator can be created."""
        evaluator = ModelEvaluator(
            judge_runner=mock_runner,
            score_store=temp_score_store
        )
        assert evaluator.gatekeeper is not None
        assert evaluator.judge is not None
        assert evaluator.selector is not None

    def test_evaluate_with_passing_response(self, mock_runner, temp_score_store, sample_good_executor_response):
        """Full evaluation with passing response."""
        # Mock the runner to return good response
        mock_runner.run.return_value = RunnerResult(
            content=sample_good_executor_response,
            success=True,
            backend="mock",
            model="mock-model"
        )

        # Judge runner returns good score
        judge_runner = MagicMock(spec=CLIRunner)
        judge_runner.name = "judge"
        judge_runner.run.return_value = RunnerResult(
            content="""
RUBRIC_SCORES:
- correctness: 8/10 - Good
- edge_cases: 7/10 - Most covered

OVERALL_SCORE: 7.5/10

STRENGTHS:
- Clean code

WEAKNESSES:
- Minor issues
""",
            success=True,
            backend="judge",
            model="judge-model"
        )

        evaluator = ModelEvaluator(
            judge_runner=judge_runner,
            score_store=temp_score_store
        )

        result = evaluator.evaluate(mock_runner, "executor")

        assert result.gatekeeper.passed is True
        assert result.judge is not None
        assert result.judge.overall_score > 0

    def test_evaluate_invalid_role(self, mock_runner, temp_score_store):
        """Raises error for invalid role."""
        evaluator = ModelEvaluator(
            judge_runner=mock_runner,
            score_store=temp_score_store
        )

        with pytest.raises(ValueError, match="Unknown role"):
            evaluator.evaluate(mock_runner, "nonexistent_role")

    def test_evaluate_runner_failure(self, mock_runner, temp_score_store):
        """Handles runner failure gracefully."""
        mock_runner.run.return_value = RunnerResult(
            content="",
            success=False,
            error="Connection failed",
            backend="mock",
            model=""
        )

        evaluator = ModelEvaluator(
            judge_runner=mock_runner,
            score_store=temp_score_store
        )

        result = evaluator.evaluate(mock_runner, "executor")

        assert result.gatekeeper.passed is False
        assert "Runner error" in result.gatekeeper.failures[0]

    def test_generate_report(self, mock_runner, temp_score_store):
        """Generates readable report."""
        evaluator = ModelEvaluator(
            judge_runner=mock_runner,
            score_store=temp_score_store
        )

        report = evaluator.generate_report()

        assert "Model Evaluation Report" in report
        assert "Leaderboard" in report


# =============================================================================
# Real Runner Tests (Optional - mark as slow)
# =============================================================================

class TestRealRunnerIntegration:
    """Tests with real runners (marked slow, skipped by default)."""

    @pytest.mark.slow
    @pytest.mark.skipif(
        not Path("/home/kblack0610/dev/home/binks-agent-orchestrator/orchestrator/cli_orchestrator/runners/groq_runner.py").exists(),
        reason="Groq runner not available"
    )
    def test_gatekeeper_with_real_groq(self, gatekeeper):
        """Test gatekeeper validation with real Groq response format."""
        # This tests that our gatekeeper handles real-world responses
        # The response format is what we'd expect from Groq
        sample_groq_response = """# Email Validation Function

Here's a complete implementation:

```python
def validate_email(email: str) -> bool:
    \"\"\"
    Validate an email address.

    Args:
        email: The email string to validate

    Returns:
        True if the email is valid, False otherwise

    Examples:
        >>> validate_email("test@example.com")
        True
        >>> validate_email("invalid")
        False
    \"\"\"
    import re

    if not email:
        return False

    pattern = r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'
    return bool(re.match(pattern, email))
```

This handles the common cases and edge cases like missing @, invalid domains, etc.
"""
        requirements = ROLE_BENCHMARKS["executor"]["requirements"]
        result = gatekeeper.validate(sample_groq_response, requirements)

        # Should pass - has code block, docstring, def, return, etc.
        assert result.passed is True, f"Failed checks: {result.failures}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
