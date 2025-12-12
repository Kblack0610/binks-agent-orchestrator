"""
QA Verifier - Stack-agnostic quality assurance agent that provides REAL verification.

This module provides a QA agent that:
  - Detects the project stack (Python, Node, Ruby, Go, Rust, etc.)
  - Runs REAL tests using the appropriate test framework
  - Collects REAL evidence (test output, screenshots, coverage)
  - REFUSES to pass without verifiable proof
  - NEVER mocks integration tests to fake success

Core Principles:
  1. REAL TESTS ONLY - No mocking integration tests to get false positives
  2. EVIDENCE REQUIRED - Show actual test output, not opinions
  3. HONEST FAILURES - If tests can't run, say so clearly
  4. STACK AGNOSTIC - Works with any language/framework
  5. SPEC COMPLIANCE - Verify against documented requirements

Usage:
    verifier = QAVerifier(project_path="/path/to/project")
    result = verifier.verify()

    if result.verified:
        print("All checks passed with evidence")
    else:
        print(f"Failed: {result.failures}")
"""
import os
import subprocess
import json
import shutil
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Any, Tuple
from pathlib import Path
from enum import Enum
from datetime import datetime


class StackType(Enum):
    """Detected project stack types."""
    PYTHON = "python"
    JAVASCRIPT = "javascript"
    TYPESCRIPT = "typescript"
    RUBY = "ruby"
    GO = "go"
    RUST = "rust"
    JAVA = "java"
    CSHARP = "csharp"
    PHP = "php"
    ELIXIR = "elixir"
    UNKNOWN = "unknown"


class TestFramework(Enum):
    """Known test frameworks by stack."""
    # Python
    PYTEST = "pytest"
    UNITTEST = "unittest"
    NOSE = "nose"
    # JavaScript/TypeScript
    JEST = "jest"
    MOCHA = "mocha"
    VITEST = "vitest"
    PLAYWRIGHT = "playwright"
    CYPRESS = "cypress"
    # Ruby
    RSPEC = "rspec"
    MINITEST = "minitest"
    # Go
    GO_TEST = "go test"
    # Rust
    CARGO_TEST = "cargo test"
    # Java
    JUNIT = "junit"
    MAVEN_TEST = "mvn test"
    GRADLE_TEST = "gradle test"
    # PHP
    PHPUNIT = "phpunit"
    # Elixir
    MIX_TEST = "mix test"
    # Generic
    UNKNOWN = "unknown"


@dataclass
class TestResult:
    """Result from running a single test suite."""
    framework: TestFramework
    command: str
    exit_code: int
    stdout: str
    stderr: str
    duration_seconds: float
    tests_run: int = 0
    tests_passed: int = 0
    tests_failed: int = 0
    tests_skipped: int = 0
    coverage_percent: Optional[float] = None

    @property
    def success(self) -> bool:
        return self.exit_code == 0

    @property
    def summary(self) -> str:
        status = "PASSED" if self.success else "FAILED"
        return f"{self.framework.value}: {status} ({self.tests_passed}/{self.tests_run} passed, {self.tests_failed} failed, {self.tests_skipped} skipped) in {self.duration_seconds:.2f}s"


@dataclass
class VerificationEvidence:
    """Evidence collected during verification."""
    test_results: List[TestResult] = field(default_factory=list)
    screenshots: List[str] = field(default_factory=list)  # Paths to screenshots
    coverage_reports: List[str] = field(default_factory=list)  # Paths to coverage files
    api_responses: List[Dict[str, Any]] = field(default_factory=list)  # API test results
    logs: List[str] = field(default_factory=list)  # Relevant log excerpts
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())

    def to_dict(self) -> Dict[str, Any]:
        return {
            "timestamp": self.timestamp,
            "test_results": [
                {
                    "framework": r.framework.value,
                    "command": r.command,
                    "exit_code": r.exit_code,
                    "success": r.success,
                    "summary": r.summary,
                    "tests_run": r.tests_run,
                    "tests_passed": r.tests_passed,
                    "tests_failed": r.tests_failed,
                    "coverage": r.coverage_percent,
                }
                for r in self.test_results
            ],
            "screenshots": self.screenshots,
            "coverage_reports": self.coverage_reports,
            "api_responses_count": len(self.api_responses),
            "logs_count": len(self.logs),
        }


@dataclass
class VerificationResult:
    """Final verification result with evidence."""
    verified: bool
    stack: StackType
    frameworks_detected: List[TestFramework]
    frameworks_run: List[TestFramework]
    evidence: VerificationEvidence
    failures: List[str] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)
    recommendations: List[str] = field(default_factory=list)

    @property
    def verdict(self) -> str:
        if self.verified:
            return "VERIFIED"
        elif self.failures:
            return "FAILED"
        else:
            return "INCONCLUSIVE"

    def summary(self) -> str:
        lines = [
            f"QA Verification Result: {self.verdict}",
            f"Stack: {self.stack.value}",
            f"Frameworks Detected: {[f.value for f in self.frameworks_detected]}",
            f"Frameworks Run: {[f.value for f in self.frameworks_run]}",
            "",
        ]

        if self.evidence.test_results:
            lines.append("Test Results:")
            for result in self.evidence.test_results:
                lines.append(f"  - {result.summary}")

        if self.failures:
            lines.append("")
            lines.append("FAILURES:")
            for f in self.failures:
                lines.append(f"  ✗ {f}")

        if self.warnings:
            lines.append("")
            lines.append("Warnings:")
            for w in self.warnings:
                lines.append(f"  ⚠ {w}")

        if self.recommendations:
            lines.append("")
            lines.append("Recommendations:")
            for r in self.recommendations:
                lines.append(f"  → {r}")

        return "\n".join(lines)


class QAVerifier:
    """
    Stack-agnostic QA verifier that runs REAL tests and collects REAL evidence.

    This is NOT a test designer - it's a test RUNNER and VERIFIER.
    It will:
      - Auto-detect your project stack
      - Find and run appropriate test frameworks
      - Collect actual test output as evidence
      - REFUSE to verify without running real tests
    """

    # Stack detection markers (file -> stack)
    STACK_MARKERS = {
        "pyproject.toml": StackType.PYTHON,
        "setup.py": StackType.PYTHON,
        "requirements.txt": StackType.PYTHON,
        "Pipfile": StackType.PYTHON,
        "package.json": StackType.JAVASCRIPT,  # Could be TS too
        "tsconfig.json": StackType.TYPESCRIPT,
        "Gemfile": StackType.RUBY,
        "go.mod": StackType.GO,
        "Cargo.toml": StackType.RUST,
        "pom.xml": StackType.JAVA,
        "build.gradle": StackType.JAVA,
        "*.csproj": StackType.CSHARP,
        "composer.json": StackType.PHP,
        "mix.exs": StackType.ELIXIR,
    }

    # Test framework detection (file/dir -> framework)
    FRAMEWORK_MARKERS = {
        # Python
        "pytest.ini": TestFramework.PYTEST,
        "pyproject.toml:pytest": TestFramework.PYTEST,
        "conftest.py": TestFramework.PYTEST,
        "test_*.py": TestFramework.PYTEST,
        "*_test.py": TestFramework.PYTEST,
        # JavaScript/TypeScript
        "jest.config.js": TestFramework.JEST,
        "jest.config.ts": TestFramework.JEST,
        "jest.config.json": TestFramework.JEST,
        "vitest.config.ts": TestFramework.VITEST,
        "vitest.config.js": TestFramework.VITEST,
        ".mocharc": TestFramework.MOCHA,
        "mocha.opts": TestFramework.MOCHA,
        "playwright.config.ts": TestFramework.PLAYWRIGHT,
        "playwright.config.js": TestFramework.PLAYWRIGHT,
        "cypress.config.js": TestFramework.CYPRESS,
        "cypress.config.ts": TestFramework.CYPRESS,
        "cypress/": TestFramework.CYPRESS,
        # Ruby
        ".rspec": TestFramework.RSPEC,
        "spec/": TestFramework.RSPEC,
        # Go
        "*_test.go": TestFramework.GO_TEST,
        # Rust
        "Cargo.toml": TestFramework.CARGO_TEST,  # Rust projects always have cargo test
        # Java
        "pom.xml": TestFramework.MAVEN_TEST,
        "build.gradle": TestFramework.GRADLE_TEST,
        # PHP
        "phpunit.xml": TestFramework.PHPUNIT,
        "phpunit.xml.dist": TestFramework.PHPUNIT,
        # Elixir
        "mix.exs": TestFramework.MIX_TEST,
    }

    # Commands to run tests for each framework
    TEST_COMMANDS = {
        TestFramework.PYTEST: ["pytest", "-v", "--tb=short"],
        TestFramework.UNITTEST: ["python", "-m", "unittest", "discover"],
        TestFramework.JEST: ["npx", "jest", "--verbose"],
        TestFramework.VITEST: ["npx", "vitest", "run"],
        TestFramework.MOCHA: ["npx", "mocha"],
        TestFramework.PLAYWRIGHT: ["npx", "playwright", "test"],
        TestFramework.CYPRESS: ["npx", "cypress", "run"],
        TestFramework.RSPEC: ["bundle", "exec", "rspec"],
        TestFramework.MINITEST: ["ruby", "-Ilib:test"],
        TestFramework.GO_TEST: ["go", "test", "./...", "-v"],
        TestFramework.CARGO_TEST: ["cargo", "test"],
        TestFramework.MAVEN_TEST: ["mvn", "test"],
        TestFramework.GRADLE_TEST: ["./gradlew", "test"],
        TestFramework.PHPUNIT: ["./vendor/bin/phpunit"],
        TestFramework.MIX_TEST: ["mix", "test"],
    }

    def __init__(
        self,
        project_path: str = ".",
        timeout: int = 300,  # 5 min default timeout per test suite
        debug: bool = False,
        venv_path: Optional[str] = None,  # Path to virtualenv for Python projects
    ):
        """
        Initialize the QA Verifier.

        Args:
            project_path: Path to the project root
            timeout: Maximum seconds to wait for test suite
            debug: Enable verbose output
            venv_path: Path to Python virtualenv (auto-detected if not provided)
        """
        self.project_path = Path(project_path).resolve()
        self.timeout = timeout
        self.debug = debug
        self.venv_path = venv_path

        # Auto-detect venv
        if not self.venv_path:
            for venv_dir in [".venv", "venv", "env", ".env"]:
                candidate = self.project_path / venv_dir
                if candidate.exists() and (candidate / "bin" / "python").exists():
                    self.venv_path = str(candidate)
                    break

    def detect_stack(self) -> StackType:
        """Detect the primary project stack."""
        for marker, stack in self.STACK_MARKERS.items():
            if marker.startswith("*"):
                # Glob pattern
                if list(self.project_path.glob(marker)):
                    return stack
            else:
                if (self.project_path / marker).exists():
                    # Special case: package.json could be TS
                    if marker == "package.json" and (self.project_path / "tsconfig.json").exists():
                        return StackType.TYPESCRIPT
                    return stack

        return StackType.UNKNOWN

    def detect_frameworks(self) -> List[TestFramework]:
        """Detect available test frameworks in the project."""
        frameworks = set()

        for marker, framework in self.FRAMEWORK_MARKERS.items():
            if ":" in marker:
                # File with content check (e.g., pyproject.toml:pytest)
                file_path, content = marker.split(":", 1)
                full_path = self.project_path / file_path
                if full_path.exists():
                    try:
                        if content in full_path.read_text():
                            frameworks.add(framework)
                    except:
                        pass
            elif marker.endswith("/"):
                # Directory check
                if (self.project_path / marker.rstrip("/")).is_dir():
                    frameworks.add(framework)
            elif "*" in marker:
                # Glob pattern - search recursively
                if list(self.project_path.rglob(marker)):
                    frameworks.add(framework)
            else:
                # Simple file existence
                if (self.project_path / marker).exists():
                    frameworks.add(framework)

        return list(frameworks)

    def _get_python_executable(self) -> str:
        """Get the Python executable, preferring venv."""
        if self.venv_path:
            venv_python = Path(self.venv_path) / "bin" / "python"
            if venv_python.exists():
                return str(venv_python)
        return "python"

    def _run_command(
        self,
        command: List[str],
        framework: TestFramework,
        env: Optional[Dict[str, str]] = None
    ) -> TestResult:
        """Run a test command and capture results."""
        import time

        start_time = time.time()

        # Prepare environment
        run_env = os.environ.copy()
        if env:
            run_env.update(env)

        # For Python projects, use venv
        if framework in [TestFramework.PYTEST, TestFramework.UNITTEST]:
            if self.venv_path:
                python = self._get_python_executable()
                if command[0] == "pytest":
                    command = [python, "-m", "pytest"] + command[1:]
                elif command[0] == "python":
                    command[0] = python

        if self.debug:
            print(f"[QA] Running: {' '.join(command)}")
            print(f"[QA] In directory: {self.project_path}")

        try:
            result = subprocess.run(
                command,
                cwd=str(self.project_path),
                capture_output=True,
                text=True,
                timeout=self.timeout,
                env=run_env,
            )

            duration = time.time() - start_time

            # Parse test counts from output (framework-specific)
            tests_run, passed, failed, skipped = self._parse_test_output(
                framework, result.stdout, result.stderr
            )

            return TestResult(
                framework=framework,
                command=" ".join(command),
                exit_code=result.returncode,
                stdout=result.stdout,
                stderr=result.stderr,
                duration_seconds=duration,
                tests_run=tests_run,
                tests_passed=passed,
                tests_failed=failed,
                tests_skipped=skipped,
            )

        except subprocess.TimeoutExpired:
            return TestResult(
                framework=framework,
                command=" ".join(command),
                exit_code=-1,
                stdout="",
                stderr=f"TIMEOUT: Test suite exceeded {self.timeout}s limit",
                duration_seconds=self.timeout,
            )
        except FileNotFoundError as e:
            return TestResult(
                framework=framework,
                command=" ".join(command),
                exit_code=-1,
                stdout="",
                stderr=f"Command not found: {e}",
                duration_seconds=0,
            )
        except Exception as e:
            return TestResult(
                framework=framework,
                command=" ".join(command),
                exit_code=-1,
                stdout="",
                stderr=f"Error running tests: {e}",
                duration_seconds=time.time() - start_time,
            )

    def _parse_test_output(
        self,
        framework: TestFramework,
        stdout: str,
        stderr: str
    ) -> Tuple[int, int, int, int]:
        """Parse test output to extract counts. Returns (total, passed, failed, skipped)."""
        import re

        output = stdout + stderr

        if framework == TestFramework.PYTEST:
            # pytest: "5 passed, 2 failed, 1 skipped in 1.23s"
            match = re.search(r"(\d+) passed", output)
            passed = int(match.group(1)) if match else 0

            match = re.search(r"(\d+) failed", output)
            failed = int(match.group(1)) if match else 0

            match = re.search(r"(\d+) skipped", output)
            skipped = int(match.group(1)) if match else 0

            match = re.search(r"(\d+) error", output)
            errors = int(match.group(1)) if match else 0
            failed += errors

            total = passed + failed + skipped
            return (total, passed, failed, skipped)

        elif framework in [TestFramework.JEST, TestFramework.VITEST]:
            # Jest: "Tests: 2 failed, 5 passed, 7 total"
            match = re.search(r"Tests:\s*(?:(\d+) failed,\s*)?(?:(\d+) passed,\s*)?(\d+) total", output)
            if match:
                failed = int(match.group(1) or 0)
                passed = int(match.group(2) or 0)
                total = int(match.group(3) or 0)
                skipped = total - passed - failed
                return (total, passed, failed, skipped)

        elif framework == TestFramework.GO_TEST:
            # Go: "ok" or "FAIL" lines, count PASS/FAIL
            passed = len(re.findall(r"--- PASS:", output))
            failed = len(re.findall(r"--- FAIL:", output))
            skipped = len(re.findall(r"--- SKIP:", output))
            total = passed + failed + skipped
            return (total, passed, failed, skipped)

        elif framework == TestFramework.RSPEC:
            # RSpec: "10 examples, 2 failures, 1 pending"
            match = re.search(r"(\d+) examples?, (\d+) failures?(?:, (\d+) pending)?", output)
            if match:
                total = int(match.group(1))
                failed = int(match.group(2))
                skipped = int(match.group(3) or 0)
                passed = total - failed - skipped
                return (total, passed, failed, skipped)

        elif framework == TestFramework.CARGO_TEST:
            # Rust: "test result: ok. 5 passed; 0 failed; 0 ignored"
            match = re.search(r"(\d+) passed; (\d+) failed; (\d+) ignored", output)
            if match:
                passed = int(match.group(1))
                failed = int(match.group(2))
                skipped = int(match.group(3))
                return (passed + failed + skipped, passed, failed, skipped)

        # Default: try to detect any pass/fail patterns
        passed = len(re.findall(r"(?i)\bpass(?:ed)?\b", output))
        failed = len(re.findall(r"(?i)\bfail(?:ed|ure)?\b", output))
        return (passed + failed, passed, failed, 0)

    def run_tests(
        self,
        frameworks: Optional[List[TestFramework]] = None,
        test_path: Optional[str] = None,
    ) -> List[TestResult]:
        """
        Run tests for specified or all detected frameworks.

        Args:
            frameworks: Specific frameworks to run (default: all detected)
            test_path: Specific path to test (optional)

        Returns:
            List of TestResult objects with actual output
        """
        if frameworks is None:
            frameworks = self.detect_frameworks()

        if not frameworks:
            return []

        results = []

        for framework in frameworks:
            if framework not in self.TEST_COMMANDS:
                continue

            command = self.TEST_COMMANDS[framework].copy()

            # Add test path if specified
            if test_path:
                command.append(test_path)

            result = self._run_command(command, framework)
            results.append(result)

            if self.debug:
                print(f"[QA] {result.summary}")

        return results

    def check_test_infrastructure(self) -> Tuple[List[str], List[str]]:
        """
        Check if proper test infrastructure exists.

        Returns:
            (warnings, recommendations) - things that are missing
        """
        warnings = []
        recommendations = []

        stack = self.detect_stack()
        frameworks = self.detect_frameworks()

        if stack == StackType.UNKNOWN:
            warnings.append("Could not detect project stack type")
            recommendations.append("Add a project manifest file (package.json, pyproject.toml, etc.)")

        if not frameworks:
            warnings.append("No test framework detected")

            if stack == StackType.PYTHON:
                recommendations.append("Add pytest: pip install pytest && touch pytest.ini")
            elif stack in [StackType.JAVASCRIPT, StackType.TYPESCRIPT]:
                recommendations.append("Add jest: npm install --save-dev jest && npx jest --init")
            elif stack == StackType.GO:
                recommendations.append("Create test files with *_test.go naming")
            elif stack == StackType.RUBY:
                recommendations.append("Add rspec: gem install rspec && rspec --init")

        # Check for test directories
        test_dirs = ["tests", "test", "spec", "__tests__", "specs"]
        has_test_dir = any((self.project_path / d).is_dir() for d in test_dirs)

        if not has_test_dir and frameworks:
            warnings.append("No standard test directory found")
            recommendations.append("Create a 'tests/' directory for test files")

        # Check for CI configuration
        ci_files = [".github/workflows", ".gitlab-ci.yml", ".circleci", "Jenkinsfile", ".travis.yml"]
        has_ci = any((self.project_path / f).exists() for f in ci_files)

        if not has_ci:
            recommendations.append("Consider adding CI configuration for automated testing")

        return warnings, recommendations

    def verify(
        self,
        spec_file: Optional[str] = None,
        require_all_pass: bool = True,
        min_coverage: Optional[float] = None,
    ) -> VerificationResult:
        """
        Run full verification and collect evidence.

        This is the main entry point. It will:
          1. Detect project stack and frameworks
          2. Check test infrastructure
          3. Run ALL detected test suites
          4. Collect evidence
          5. Return honest verdict

        Args:
            spec_file: Optional path to spec/requirements file to verify against
            require_all_pass: Require all tests to pass (default: True)
            min_coverage: Minimum coverage percentage required (optional)

        Returns:
            VerificationResult with evidence and honest verdict
        """
        stack = self.detect_stack()
        frameworks = self.detect_frameworks()

        evidence = VerificationEvidence()
        failures = []

        # Check infrastructure
        infra_warnings, recommendations = self.check_test_infrastructure()

        if not frameworks:
            failures.append("NO TEST FRAMEWORK DETECTED - Cannot verify without tests")
            return VerificationResult(
                verified=False,
                stack=stack,
                frameworks_detected=[],
                frameworks_run=[],
                evidence=evidence,
                failures=failures,
                warnings=infra_warnings,
                recommendations=recommendations,
            )

        # Run tests
        test_results = self.run_tests(frameworks)
        evidence.test_results = test_results

        frameworks_run = [r.framework for r in test_results]

        # Analyze results
        all_passed = all(r.success for r in test_results)
        any_run = len(test_results) > 0

        if not any_run:
            failures.append("No tests were actually executed")

        for result in test_results:
            if not result.success:
                failures.append(f"{result.framework.value}: {result.tests_failed} tests failed")
                if result.stderr and "not found" in result.stderr.lower():
                    failures.append(f"{result.framework.value}: Test command not available")

        # Check coverage if requested
        if min_coverage is not None:
            for result in test_results:
                if result.coverage_percent is not None:
                    if result.coverage_percent < min_coverage:
                        failures.append(
                            f"{result.framework.value}: Coverage {result.coverage_percent}% < required {min_coverage}%"
                        )

        # Determine verdict
        verified = (
            any_run and
            (all_passed if require_all_pass else True) and
            len(failures) == 0
        )

        return VerificationResult(
            verified=verified,
            stack=stack,
            frameworks_detected=frameworks,
            frameworks_run=frameworks_run,
            evidence=evidence,
            failures=failures,
            warnings=infra_warnings,
            recommendations=recommendations,
        )

    def verify_spec_compliance(
        self,
        spec_items: List[str],
        test_results: List[TestResult],
    ) -> List[Tuple[str, bool, str]]:
        """
        Check if test output indicates spec items are verified.

        Args:
            spec_items: List of spec/requirement descriptions
            test_results: Test results to analyze

        Returns:
            List of (spec_item, verified, evidence) tuples
        """
        # This is a simple keyword-based check
        # A more sophisticated version would use LLM to analyze
        results = []

        all_output = "\n".join(
            r.stdout + r.stderr for r in test_results
        ).lower()

        for spec in spec_items:
            # Extract keywords from spec
            keywords = [w.lower() for w in spec.split() if len(w) > 3]

            # Check if keywords appear in test output
            matches = sum(1 for kw in keywords if kw in all_output)
            coverage = matches / len(keywords) if keywords else 0

            verified = coverage > 0.5  # At least half the keywords found
            evidence = f"Found {matches}/{len(keywords)} keywords in test output"

            results.append((spec, verified, evidence))

        return results


# =============================================================================
# QA Agent System Prompt
# =============================================================================

QA_VERIFIER_PROMPT = """You are a rigorous QA Verification Agent.

YOUR CORE PRINCIPLES:
1. NEVER verify without REAL test evidence
2. NEVER accept mocked integration tests as proof
3. ALWAYS run actual test commands
4. ALWAYS show raw test output as evidence
5. HONESTLY report when tests cannot be run

YOU ARE NOT A CODE REVIEWER. You are a TEST RUNNER and VERIFIER.

When asked to verify a feature/project:

1. DETECT the project stack (Python, Node, Ruby, Go, etc.)
2. IDENTIFY available test frameworks (pytest, jest, rspec, etc.)
3. RUN the actual tests using appropriate commands
4. COLLECT real output as evidence
5. REPORT results honestly

YOUR OUTPUT FORMAT:
```
VERIFICATION REPORT
==================

Stack Detected: [Python/Node/Ruby/etc.]
Test Frameworks Found: [pytest, jest, etc.]

TESTS EXECUTED:
--------------
Command: [actual command run]
Exit Code: [0 or error code]
Output:
[ACTUAL RAW OUTPUT FROM TESTS]

EVIDENCE SUMMARY:
- Tests Run: X
- Passed: Y
- Failed: Z
- Skipped: W

VERDICT: [VERIFIED / FAILED / INCONCLUSIVE]

FAILURES (if any):
- [List specific failures with evidence]

RECOMMENDATIONS:
- [List if tests couldn't run or coverage is low]
```

IF TESTS CANNOT RUN:
- Clearly state WHY (missing framework, no tests, dependency issues)
- DO NOT claim verification
- Provide setup recommendations

NEVER:
- Say "tests look good" without running them
- Mock data to get passing results
- Verify without actual test execution
- Claim coverage without running coverage tools
"""


# =============================================================================
# Convenience Functions
# =============================================================================

def create_qa_verifier(
    project_path: str = ".",
    debug: bool = False,
) -> QAVerifier:
    """Create a QA verifier for a project."""
    return QAVerifier(project_path=project_path, debug=debug)


def quick_verify(project_path: str = ".") -> VerificationResult:
    """Quick verification of a project."""
    verifier = QAVerifier(project_path=project_path)
    return verifier.verify()


# =============================================================================
# CLI Interface
# =============================================================================

if __name__ == "__main__":
    import sys

    path = sys.argv[1] if len(sys.argv) > 1 else "."
    debug = "--debug" in sys.argv or "-d" in sys.argv

    print(f"QA Verifier - Verifying: {path}")
    print("=" * 50)

    verifier = QAVerifier(project_path=path, debug=debug)
    result = verifier.verify()

    print(result.summary())
    print()

    if result.evidence.test_results:
        print("Raw Test Output:")
        print("-" * 40)
        for tr in result.evidence.test_results:
            print(f"\n[{tr.framework.value}]")
            if tr.stdout:
                print(tr.stdout[:2000])  # Truncate for display
            if tr.stderr:
                print("STDERR:", tr.stderr[:500])

    sys.exit(0 if result.verified else 1)
