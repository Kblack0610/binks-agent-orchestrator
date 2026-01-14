"""
Meritocratic Model Evaluation System

A performance-based model selection system that:
1. Benchmarks models per role using standardized prompts
2. Evaluates responses with Gatekeeper (heuristic) + Judge (LLM)
3. Stores scores for smart model selection

Philosophy:
- NO FALLBACK - just pick the best tool for the job
- Meritocratic selection based on proven performance
- "Hiring Phase" (benchmark) vs "Working Phase" (runtime)

Architecture:
    ROLE_BENCHMARKS → Runner.run() → Gatekeeper.validate() → Judge.score() → ScoreStore → ModelSelector
"""
import json
import re
import time
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Dict, List, Optional, Any, Callable, Tuple

# Handle imports for both module and standalone usage
try:
    from .runners.base import CLIRunner, RunnerResult
except ImportError:
    from runners.base import CLIRunner, RunnerResult


# =============================================================================
# Role Benchmarks - Standardized Test Prompts Per Role
# =============================================================================

ROLE_BENCHMARKS: Dict[str, Dict[str, Any]] = {
    "architect": {
        "prompt": """Design a REST API for a user management system.

Requirements:
- User CRUD operations (create, read, update, delete)
- Authentication endpoints (login, logout, refresh)
- Role-based access control (admin, user, guest)

Provide:
1. Endpoint specifications (method, path, request/response)
2. Data models
3. Security considerations
4. Scalability notes""",
        "requirements": {
            "min_length": 500,
            "must_contain": ["endpoint", "POST", "GET", "authentication", "security"],
            "must_have_structure": True,  # Headers, lists, code blocks
            "code_blocks_min": 1,
        },
        "rubric": {
            "completeness": "Does it cover all CRUD + auth endpoints?",
            "clarity": "Are the specs clear and actionable?",
            "security": "Are security considerations addressed?",
            "scalability": "Does it mention scalability/performance?",
        }
    },

    "executor": {
        "prompt": """Implement a Python function that validates email addresses.

Requirements:
- Accept a string, return True if valid email, False otherwise
- Handle common edge cases (missing @, invalid domains, etc.)
- Include docstring with examples
- Include type hints

Provide the complete implementation.""",
        "requirements": {
            "min_length": 200,
            "must_contain": ["def ", "return", ":", "@"],
            "code_blocks_min": 1,
            "has_docstring": True,
        },
        "rubric": {
            "correctness": "Does the function correctly validate emails?",
            "edge_cases": "Does it handle edge cases (missing @, spaces, etc.)?",
            "code_quality": "Is it clean, readable, well-documented?",
            "type_hints": "Does it use proper type hints?",
        }
    },

    "critic": {
        "prompt": """Review the following Python code for issues:

```python
def process_user(user_data):
    name = user_data['name']
    email = user_data['email']

    if email.contains('@'):
        return {'status': 'valid', 'user': name}
    else:
        return {'status': 'invalid'}

def get_users():
    users = open('users.txt').read()
    return eval(users)
```

Provide a thorough code review with:
1. Identified issues (bugs, security, style)
2. Severity of each issue
3. Specific fix recommendations

End with: VERDICT: PASS or VERDICT: FAIL""",
        "requirements": {
            "min_length": 300,
            "must_contain": ["VERDICT"],
            "issues_found_min": 3,  # Should find: .contains→in, no error handling, eval security
            "has_recommendations": True,
        },
        "rubric": {
            "bug_detection": "Did it catch the .contains() bug?",
            "security_awareness": "Did it flag eval() as dangerous?",
            "resource_handling": "Did it note missing file close/context manager?",
            "actionable_feedback": "Are fixes specific and actionable?",
        }
    },

    "verifier": {
        "prompt": """You have a Python project with this structure:

```
myproject/
├── pyproject.toml
├── src/
│   └── calculator.py
└── tests/
    └── test_calculator.py
```

The calculator.py has functions: add, subtract, multiply, divide.
The test_calculator.py has tests for each function.

Describe exactly how you would verify this project works:
1. What commands would you run?
2. What evidence would you collect?
3. How would you report the results?

Be specific about actual commands and expected output.""",
        "requirements": {
            "min_length": 300,
            "must_contain": ["pytest", "command", "output", "evidence"],
            "mentions_real_commands": True,
            "no_fake_results": True,  # Should NOT claim tests passed without running
        },
        "rubric": {
            "realism": "Does it describe running ACTUAL tests, not mocking?",
            "specificity": "Are commands specific and correct?",
            "evidence_focus": "Does it emphasize collecting real output?",
            "honesty": "Does it avoid claiming results without evidence?",
        }
    },

    "researcher": {
        "prompt": """Research the trade-offs between REST and GraphQL for a mobile app backend.

Consider:
- Performance implications
- Development complexity
- Caching strategies
- Mobile-specific concerns (bandwidth, battery)

Provide a balanced analysis with clear recommendations.""",
        "requirements": {
            "min_length": 400,
            "must_contain": ["REST", "GraphQL", "trade-off", "recommend"],
            "covers_both_sides": True,
            "has_conclusion": True,
        },
        "rubric": {
            "balance": "Does it fairly present both options?",
            "depth": "Does it go beyond surface-level comparison?",
            "mobile_focus": "Does it address mobile-specific concerns?",
            "actionable": "Is the recommendation clear and justified?",
        }
    },

    "debugger": {
        "prompt": """Debug this error:

```
Traceback (most recent call last):
  File "app.py", line 45, in process_request
    result = handler.process(data)
  File "handlers.py", line 23, in process
    return self.transform(self.validate(data))
  File "handlers.py", line 31, in validate
    if data['user_id'] > 0:
TypeError: '>' not supported between instances of 'str' and 'int'
```

Context: This is a web API that receives JSON requests. The error happens
intermittently - some requests work, some fail.

Provide:
1. Root cause analysis
2. Why it's intermittent
3. The fix
4. Prevention strategies""",
        "requirements": {
            "min_length": 250,
            "must_contain": ["type", "string", "int", "convert"],
            "identifies_root_cause": True,
            "explains_intermittent": True,
        },
        "rubric": {
            "diagnosis": "Does it correctly identify the type mismatch?",
            "intermittent_explanation": "Does it explain why some requests work?",
            "fix_quality": "Is the fix correct and complete?",
            "prevention": "Does it suggest validation/type checking?",
        }
    },

    "tester": {
        "prompt": """Write test cases for a shopping cart class with methods:
- add_item(item_id, quantity)
- remove_item(item_id)
- get_total()
- apply_discount(code)
- checkout()

Provide comprehensive test cases covering:
1. Happy paths
2. Edge cases
3. Error conditions

Use pytest syntax.""",
        "requirements": {
            "min_length": 400,
            "must_contain": ["def test_", "assert", "pytest"],
            "code_blocks_min": 1,
            "test_count_min": 5,
        },
        "rubric": {
            "coverage": "Does it cover all methods?",
            "edge_cases": "Does it test empty cart, invalid items, etc.?",
            "error_handling": "Does it test error conditions?",
            "readability": "Are tests clear and well-named?",
        }
    },

    "documenter": {
        "prompt": """Document this Python class:

```python
class RateLimiter:
    def __init__(self, max_requests, window_seconds):
        self.max_requests = max_requests
        self.window = window_seconds
        self.requests = {}

    def allow(self, client_id):
        now = time.time()
        self._cleanup(client_id, now)

        if client_id not in self.requests:
            self.requests[client_id] = []

        if len(self.requests[client_id]) >= self.max_requests:
            return False

        self.requests[client_id].append(now)
        return True

    def _cleanup(self, client_id, now):
        if client_id in self.requests:
            self.requests[client_id] = [
                t for t in self.requests[client_id]
                if now - t < self.window
            ]
```

Provide:
1. Class docstring explaining purpose and usage
2. Method docstrings with args, returns, examples
3. Usage example showing common patterns""",
        "requirements": {
            "min_length": 400,
            "must_contain": ["Args:", "Returns:", "Example", '"""'],
            "has_usage_example": True,
            "explains_algorithm": True,
        },
        "rubric": {
            "clarity": "Is the documentation clear for new users?",
            "completeness": "Are all methods documented?",
            "examples": "Are examples practical and runnable?",
            "accuracy": "Does it correctly describe the behavior?",
        }
    },

    "planner": {
        "prompt": """Plan the implementation of a notification system for a web app.

User's request: "I need to add notifications to my app. Users should get
notified when someone comments on their posts."

Before implementation, clarify:
1. What requirements need to be gathered?
2. What questions would you ask the user?
3. What are the key implementation decisions?
4. What's the proposed implementation order?

Be thorough but practical.""",
        "requirements": {
            "min_length": 400,
            "must_contain": ["question", "requirement", "decision", "step"],
            "asks_clarifying_questions": True,
            "has_implementation_order": True,
        },
        "rubric": {
            "requirements_gathering": "Does it identify what info is missing?",
            "question_quality": "Are questions specific and useful?",
            "decision_mapping": "Does it outline key technical decisions?",
            "actionable_plan": "Is the implementation plan clear and ordered?",
        }
    },

    "triage": {
        "prompt": """You are a workflow router. For each task below, select the appropriate workflow.

AVAILABLE WORKFLOWS:
- QUICK: No agents needed. Direct answer for simple questions.
- SIMPLE: executor only. Small, well-defined tasks.
- STANDARD: architect → executor → verifier → critic. Features that need design.
- FULL: planner → architect → executor → verifier → critic → gatekeeper → judge. Complex/unclear requirements.
- DEBUG: debugger → executor → verifier. Bug reports, errors.
- RESEARCH: researcher → documenter. Information gathering, documentation.
- REVIEW: critic → gatekeeper → judge. PR reviews, code audits.
- TEST: tester → executor → verifier. Test creation, coverage.

TASKS TO ROUTE:
1. "What is 2+2?"
2. "Build authentication system with OAuth, JWT, and role-based access control"
3. "Add a print statement to debug this function"
4. "Why is this test failing? Here's the stack trace: TypeError..."
5. "How does the React useEffect hook work? When should I use it?"

For EACH task, respond with a JSON object:
{
  "task": 1,
  "workflow": "WORKFLOW_NAME",
  "roles": ["role1", "role2"],
  "reasoning": "Brief explanation"
}

Expected routing:
- Task 1: QUICK (simple math)
- Task 2: FULL (complex system, needs planning)
- Task 3: SIMPLE (small change)
- Task 4: DEBUG (bug investigation)
- Task 5: RESEARCH (information gathering)""",
        "requirements": {
            "min_length": 400,
            "must_contain": ["workflow", "roles", "reasoning"],
            "valid_json_objects": 5,  # Should output 5 JSON objects
            "correct_workflow_mapping": {
                1: "QUICK",
                2: "FULL",
                3: "SIMPLE",
                4: "DEBUG",
                5: "RESEARCH",
            }
        },
        "rubric": {
            "workflow_accuracy": "Does it select the correct workflow for each task?",
            "role_accuracy": "Are the role sequences correct for each workflow?",
            "reasoning_quality": "Is the reasoning logical and explains the choice?",
            "json_format": "Does it output valid JSON as specified?",
        }
    },

    "gatekeeper": {
        "prompt": """You are validating a response against these requirements:
- Minimum 200 characters
- Must contain: "function", "return", "parameter"
- Must have at least one code block

Response to validate:
'''
Here's a simple function:

```python
def add(a, b):
    return a + b
```

This function takes two parameters and returns their sum.
'''

Provide your validation result.""",
        "requirements": {
            "min_length": 100,
            "must_contain": ["GATE", "PASS", "check"],
            "has_validation_output": True,
        },
        "rubric": {
            "accuracy": "Does it correctly identify what passes/fails?",
            "specificity": "Does it list specific checks performed?",
            "format": "Does it follow the expected output format?",
            "completeness": "Does it check all requirements?",
        }
    },

    "judge": {
        "prompt": """Evaluate this code review response using these criteria:
- Bug detection: Did it find issues?
- Actionable feedback: Are suggestions specific?
- Professionalism: Is the tone appropriate?

Response to evaluate:
'''
This code has several issues:
1. Using eval() is a security vulnerability
2. The file is never closed (should use context manager)
3. .contains() should be "in" operator

Recommendations:
- Replace eval() with json.loads()
- Use "with open(...) as f:"
- Change to: if '@' in email

VERDICT: FAIL
'''

Score each criterion 1-10 and provide overall assessment.""",
        "requirements": {
            "min_length": 200,
            "must_contain": ["score", "OVERALL", "/10"],
            "has_per_criterion_scores": True,
        },
        "rubric": {
            "objectivity": "Are scores fair and justified?",
            "consistency": "Do scores align with the feedback given?",
            "format_adherence": "Does it follow the scoring format?",
            "actionable_feedback": "Does it provide useful meta-feedback?",
        }
    },
}


# =============================================================================
# Data Classes
# =============================================================================

@dataclass
class GatekeeperResult:
    """Result from heuristic validation."""
    passed: bool
    checks: Dict[str, bool]
    failures: List[str]
    score: float  # 0.0-1.0 based on checks passed

    def __str__(self) -> str:
        status = "PASS" if self.passed else "FAIL"
        return f"Gatekeeper: {status} ({self.score:.0%}) - {len(self.failures)} issues"


@dataclass
class JudgeResult:
    """Result from LLM quality evaluation."""
    scores: Dict[str, float]  # rubric item → score (1-10)
    overall_score: float  # 1-10 average
    feedback: str
    strengths: List[str]
    weaknesses: List[str]

    def __str__(self) -> str:
        return f"Judge: {self.overall_score:.1f}/10 - {len(self.strengths)} strengths, {len(self.weaknesses)} weaknesses"


@dataclass
class EvaluationResult:
    """Complete evaluation result for a model on a role."""
    role: str
    model: str
    backend: str
    gatekeeper: GatekeeperResult
    judge: Optional[JudgeResult]
    response: str
    execution_time: float
    timestamp: datetime = field(default_factory=datetime.now)

    @property
    def passed(self) -> bool:
        """Overall pass: gatekeeper passed AND judge score >= 6."""
        if not self.gatekeeper.passed:
            return False
        if self.judge and self.judge.overall_score < 6.0:
            return False
        return True

    @property
    def final_score(self) -> float:
        """Combined score: gatekeeper (30%) + judge (70%)."""
        gate_score = self.gatekeeper.score * 10  # Convert to 1-10 scale
        judge_score = self.judge.overall_score if self.judge else 5.0
        return (gate_score * 0.3) + (judge_score * 0.7)

    def to_dict(self) -> dict:
        return {
            "role": self.role,
            "model": self.model,
            "backend": self.backend,
            "passed": self.passed,
            "final_score": self.final_score,
            "gatekeeper_score": self.gatekeeper.score,
            "judge_score": self.judge.overall_score if self.judge else None,
            "execution_time": self.execution_time,
            "timestamp": self.timestamp.isoformat(),
        }


# =============================================================================
# Gatekeeper - Deterministic Heuristic Validation
# =============================================================================

class Gatekeeper:
    """
    Fast, deterministic validation of responses.

    Cannot hallucinate - uses pure Python checks.
    Runs BEFORE the expensive LLM judge.
    """

    def validate(self, response: str, requirements: Dict[str, Any]) -> GatekeeperResult:
        """
        Validate response against requirements.

        Args:
            response: The model's response text
            requirements: Dict of requirements from ROLE_BENCHMARKS

        Returns:
            GatekeeperResult with pass/fail and details
        """
        checks = {}
        failures = []

        # Check minimum length
        if "min_length" in requirements:
            min_len = requirements["min_length"]
            checks["min_length"] = len(response) >= min_len
            if not checks["min_length"]:
                failures.append(f"Response too short: {len(response)} < {min_len}")

        # Check required content
        if "must_contain" in requirements:
            for term in requirements["must_contain"]:
                key = f"contains_{term}"
                checks[key] = term.lower() in response.lower()
                if not checks[key]:
                    failures.append(f"Missing required term: '{term}'")

        # Check for code blocks
        if "code_blocks_min" in requirements:
            code_block_count = response.count("```")
            # Code blocks come in pairs
            actual_blocks = code_block_count // 2
            min_blocks = requirements["code_blocks_min"]
            checks["code_blocks"] = actual_blocks >= min_blocks
            if not checks["code_blocks"]:
                failures.append(f"Not enough code blocks: {actual_blocks} < {min_blocks}")

        # Check for structure (headers, lists)
        if requirements.get("must_have_structure"):
            has_headers = bool(re.search(r'^#+\s', response, re.MULTILINE))
            has_lists = bool(re.search(r'^[-*]\s|^\d+[.)]\s', response, re.MULTILINE))
            checks["has_structure"] = has_headers or has_lists
            if not checks["has_structure"]:
                failures.append("Missing structure (headers or lists)")

        # Check for docstring (Python-specific)
        if requirements.get("has_docstring"):
            checks["has_docstring"] = '"""' in response or "'''" in response
            if not checks["has_docstring"]:
                failures.append("Missing docstring")

        # Check for verdict (critic role)
        if "VERDICT" in str(requirements.get("must_contain", [])):
            has_verdict = "VERDICT:" in response.upper() or "VERDICT :" in response.upper()
            checks["has_verdict"] = has_verdict
            if not checks["has_verdict"]:
                failures.append("Missing VERDICT declaration")

        # Check minimum test count (tester role)
        if "test_count_min" in requirements:
            test_count = len(re.findall(r'def test_\w+', response))
            min_tests = requirements["test_count_min"]
            checks["test_count"] = test_count >= min_tests
            if not checks["test_count"]:
                failures.append(f"Not enough tests: {test_count} < {min_tests}")

        # Check for issues found (critic role)
        if "issues_found_min" in requirements:
            # Count issue indicators
            issue_patterns = [
                r'issue[s]?:',
                r'problem[s]?:',
                r'bug[s]?:',
                r'\d+\.',  # Numbered lists
                r'- \*\*',  # Bold list items
            ]
            issue_count = sum(
                len(re.findall(p, response, re.IGNORECASE))
                for p in issue_patterns
            )
            min_issues = requirements["issues_found_min"]
            checks["issues_found"] = issue_count >= min_issues
            if not checks["issues_found"]:
                failures.append(f"Not enough issues identified: ~{issue_count} < {min_issues}")

        # Check for clarifying questions (planner role)
        if requirements.get("asks_clarifying_questions"):
            question_count = response.count("?")
            checks["asks_questions"] = question_count >= 3
            if not checks["asks_questions"]:
                failures.append(f"Not enough clarifying questions: {question_count} < 3")

        # Calculate score
        if checks:
            score = sum(1 for v in checks.values() if v) / len(checks)
        else:
            score = 1.0  # No requirements = pass

        # Pass if score >= 70% of checks
        passed = score >= 0.7 and len(failures) <= 2

        return GatekeeperResult(
            passed=passed,
            checks=checks,
            failures=failures,
            score=score
        )


# =============================================================================
# Judge - LLM Quality Evaluation
# =============================================================================

class Judge:
    """
    LLM-based quality evaluation using role-specific rubrics.

    Uses a fixed, high-quality model to ensure consistent evaluation.
    Only runs if Gatekeeper passes (saves cost on obviously bad responses).
    """

    JUDGE_PROMPT_TEMPLATE = """You are a technical evaluator. Score this response objectively.

ROLE: {role}
TASK: {task}
RESPONSE TO EVALUATE:
{response}

EVALUATION RUBRIC:
{rubric}

For each rubric item, provide:
1. Score (1-10, where 10 is excellent)
2. Brief justification (1 sentence)

Then provide:
- OVERALL_SCORE: (average of all scores)
- STRENGTHS: (2-3 bullet points)
- WEAKNESSES: (2-3 bullet points)

Format your response EXACTLY as:
RUBRIC_SCORES:
- {rubric_item_1}: X/10 - justification
- {rubric_item_2}: X/10 - justification
...

OVERALL_SCORE: X.X/10

STRENGTHS:
- strength 1
- strength 2

WEAKNESSES:
- weakness 1
- weakness 2"""

    def __init__(self, judge_runner: CLIRunner, debug: bool = False):
        """
        Initialize Judge with a runner for evaluation.

        Args:
            judge_runner: The LLM runner to use for judging (should be high quality)
            debug: Enable debug output
        """
        self.runner = judge_runner
        self.debug = debug

    def evaluate(
        self,
        response: str,
        role: str,
        benchmark: Dict[str, Any]
    ) -> JudgeResult:
        """
        Evaluate a response using the role's rubric.

        Args:
            response: The model's response to evaluate
            role: The role being evaluated
            benchmark: The benchmark dict from ROLE_BENCHMARKS

        Returns:
            JudgeResult with scores and feedback
        """
        rubric = benchmark.get("rubric", {})
        task = benchmark.get("prompt", "")[:200] + "..."

        # Format rubric for prompt
        rubric_text = "\n".join(f"- {k}: {v}" for k, v in rubric.items())
        rubric_items = "\n".join(f"- {k}: X/10 - justification" for k in rubric.keys())

        prompt = self.JUDGE_PROMPT_TEMPLATE.format(
            role=role,
            task=task,
            response=response[:3000],  # Truncate long responses
            rubric=rubric_text,
            rubric_item_1=list(rubric.keys())[0] if rubric else "quality",
            rubric_item_2=list(rubric.keys())[1] if len(rubric) > 1 else "completeness",
        )

        if self.debug:
            print(f"[Judge] Evaluating {role} response ({len(response)} chars)")

        result = self.runner.run(prompt)

        if not result.success:
            # Return default scores on failure
            return JudgeResult(
                scores={k: 5.0 for k in rubric},
                overall_score=5.0,
                feedback=f"Judge evaluation failed: {result.error}",
                strengths=["Could not evaluate"],
                weaknesses=["Judge runner failed"]
            )

        return self._parse_judge_response(result.content, rubric)

    def _parse_judge_response(
        self,
        content: str,
        rubric: Dict[str, str]
    ) -> JudgeResult:
        """Parse the judge's response into structured results."""
        scores = {}
        strengths = []
        weaknesses = []
        overall_score = 5.0

        # Parse rubric scores
        for key in rubric.keys():
            # Look for patterns like "key: 8/10" or "key: 8 /10"
            pattern = rf'{re.escape(key)}[:\s]+(\d+(?:\.\d+)?)\s*/\s*10'
            match = re.search(pattern, content, re.IGNORECASE)
            if match:
                scores[key] = float(match.group(1))
            else:
                scores[key] = 5.0  # Default

        # Parse overall score
        overall_match = re.search(r'OVERALL[_\s]SCORE[:\s]+(\d+(?:\.\d+)?)', content, re.IGNORECASE)
        if overall_match:
            overall_score = float(overall_match.group(1))
        elif scores:
            overall_score = sum(scores.values()) / len(scores)

        # Parse strengths
        strengths_section = re.search(r'STRENGTHS?:(.*?)(?:WEAKNESSES?:|$)', content, re.IGNORECASE | re.DOTALL)
        if strengths_section:
            strengths = re.findall(r'[-•]\s*(.+)', strengths_section.group(1))
            strengths = [s.strip() for s in strengths[:3]]

        # Parse weaknesses
        weaknesses_section = re.search(r'WEAKNESSES?:(.*?)$', content, re.IGNORECASE | re.DOTALL)
        if weaknesses_section:
            weaknesses = re.findall(r'[-•]\s*(.+)', weaknesses_section.group(1))
            weaknesses = [w.strip() for w in weaknesses[:3]]

        return JudgeResult(
            scores=scores,
            overall_score=min(10.0, max(1.0, overall_score)),  # Clamp to 1-10
            feedback=content,
            strengths=strengths or ["No specific strengths noted"],
            weaknesses=weaknesses or ["No specific weaknesses noted"]
        )


# =============================================================================
# Score Store - Persistence for Model Scores
# =============================================================================

class ScoreStore:
    """
    JSON-based persistence for model evaluation scores.

    Stores scores per role per model for smart selection.
    """

    def __init__(self, storage_path: Optional[Path] = None):
        """
        Initialize score store.

        Args:
            storage_path: Path to JSON file (default: ~/.cli_orchestrator/model_scores.json)
        """
        if storage_path is None:
            storage_path = Path.home() / ".cli_orchestrator" / "model_scores.json"

        self.storage_path = storage_path
        self.storage_path.parent.mkdir(parents=True, exist_ok=True)
        self._scores = self._load()

    def _load(self) -> Dict[str, Dict[str, Any]]:
        """Load scores from disk."""
        if self.storage_path.exists():
            try:
                with open(self.storage_path) as f:
                    return json.load(f)
            except (json.JSONDecodeError, IOError):
                return {}
        return {}

    def _save(self) -> None:
        """Save scores to disk."""
        with open(self.storage_path, 'w') as f:
            json.dump(self._scores, f, indent=2, default=str)

    def record(self, result: EvaluationResult) -> None:
        """
        Record an evaluation result.

        Args:
            result: The evaluation result to store
        """
        key = f"{result.backend}:{result.model}"

        if key not in self._scores:
            self._scores[key] = {
                "backend": result.backend,
                "model": result.model,
                "roles": {},
                "last_updated": None
            }

        self._scores[key]["roles"][result.role] = {
            "score": result.final_score,
            "gatekeeper_score": result.gatekeeper.score,
            "judge_score": result.judge.overall_score if result.judge else None,
            "passed": result.passed,
            "execution_time": result.execution_time,
            "timestamp": result.timestamp.isoformat()
        }
        self._scores[key]["last_updated"] = datetime.now().isoformat()

        self._save()

    def get_score(self, backend: str, model: str, role: str) -> Optional[float]:
        """Get stored score for a model on a role."""
        key = f"{backend}:{model}"
        if key in self._scores and role in self._scores[key].get("roles", {}):
            return self._scores[key]["roles"][role]["score"]
        return None

    def get_best_for_role(self, role: str) -> Optional[Tuple[str, str, float]]:
        """
        Get the best model for a role.

        Returns:
            Tuple of (backend, model, score) or None if no data
        """
        best = None
        best_score = -1

        for key, data in self._scores.items():
            if role in data.get("roles", {}):
                score = data["roles"][role]["score"]
                if score > best_score:
                    best_score = score
                    best = (data["backend"], data["model"], score)

        return best

    def get_all_scores(self) -> Dict[str, Dict[str, Any]]:
        """Get all stored scores."""
        return self._scores.copy()

    def get_leaderboard(self, role: str) -> List[Dict[str, Any]]:
        """Get ranked list of models for a role."""
        entries = []

        for key, data in self._scores.items():
            if role in data.get("roles", {}):
                entries.append({
                    "backend": data["backend"],
                    "model": data["model"],
                    **data["roles"][role]
                })

        return sorted(entries, key=lambda x: x["score"], reverse=True)


# =============================================================================
# Model Selector - Smart Model Selection
# =============================================================================

class ModelSelector:
    """
    Selects the best model for a role based on scores.

    Supports:
    - Automatic selection based on benchmark scores
    - Config overrides for explicit mappings
    - Fallback to specified defaults (NOT cascading fallback)
    """

    def __init__(
        self,
        score_store: ScoreStore,
        config_overrides: Optional[Dict[str, str]] = None,
        default_backend: str = "claude",
        default_model: str = "claude-sonnet-4-20250514"
    ):
        """
        Initialize model selector.

        Args:
            score_store: ScoreStore with benchmark results
            config_overrides: Explicit role → "backend:model" mappings
            default_backend: Default backend if no scores available
            default_model: Default model if no scores available
        """
        self.score_store = score_store
        self.config_overrides = config_overrides or {}
        self.default_backend = default_backend
        self.default_model = default_model

    def select(self, role: str, min_score: float = 6.0) -> Tuple[str, str]:
        """
        Select the best model for a role.

        Args:
            role: The role to select a model for
            min_score: Minimum acceptable score (default 6.0)

        Returns:
            Tuple of (backend, model)
        """
        # 1. Check config overrides first
        if role in self.config_overrides:
            override = self.config_overrides[role]
            if ":" in override:
                backend, model = override.split(":", 1)
                return (backend, model)

        # 2. Check benchmark scores
        best = self.score_store.get_best_for_role(role)
        if best and best[2] >= min_score:
            return (best[0], best[1])

        # 3. Return default (NOT fallback to next best)
        return (self.default_backend, self.default_model)

    def get_selection_reason(self, role: str) -> str:
        """Explain why a model was selected."""
        if role in self.config_overrides:
            return f"Config override: {self.config_overrides[role]}"

        best = self.score_store.get_best_for_role(role)
        if best:
            return f"Best benchmark score: {best[0]}:{best[1]} ({best[2]:.1f}/10)"

        return f"Default: {self.default_backend}:{self.default_model} (no benchmarks)"


# =============================================================================
# Main Evaluator - Orchestrates the Evaluation Pipeline
# =============================================================================

class ModelEvaluator:
    """
    Main evaluation pipeline that orchestrates benchmarking.

    Usage:
        evaluator = ModelEvaluator(judge_runner=ClaudeRunner())

        # Evaluate a single model on a role
        result = evaluator.evaluate(GroqRunner(), "executor")

        # Benchmark all roles
        results = evaluator.benchmark_all_roles(GroqRunner())

        # Get best model for a role
        backend, model = evaluator.selector.select("architect")
    """

    def __init__(
        self,
        judge_runner: CLIRunner,
        score_store: Optional[ScoreStore] = None,
        config_overrides: Optional[Dict[str, str]] = None,
        debug: bool = False
    ):
        """
        Initialize the evaluator.

        Args:
            judge_runner: High-quality runner for the Judge
            score_store: Storage for scores (default: creates new)
            config_overrides: Explicit role → model mappings
            debug: Enable debug output
        """
        self.gatekeeper = Gatekeeper()
        self.judge = Judge(judge_runner, debug=debug)
        self.score_store = score_store or ScoreStore()
        self.selector = ModelSelector(
            self.score_store,
            config_overrides=config_overrides
        )
        self.debug = debug

    def evaluate(
        self,
        runner: CLIRunner,
        role: str,
        skip_judge_on_gate_fail: bool = True
    ) -> EvaluationResult:
        """
        Evaluate a runner on a specific role.

        Args:
            runner: The runner to evaluate
            role: Role to evaluate (must be in ROLE_BENCHMARKS)
            skip_judge_on_gate_fail: Skip expensive judge if gatekeeper fails

        Returns:
            EvaluationResult with complete evaluation data
        """
        if role not in ROLE_BENCHMARKS:
            raise ValueError(f"Unknown role: {role}. Available: {list(ROLE_BENCHMARKS.keys())}")

        benchmark = ROLE_BENCHMARKS[role]
        prompt = benchmark["prompt"]
        requirements = benchmark["requirements"]

        if self.debug:
            print(f"[Evaluator] Testing {runner.name} on {role}")

        # Run the model
        start_time = time.time()
        result = runner.run(prompt)
        execution_time = time.time() - start_time

        if not result.success:
            # Model failed to respond
            gate_result = GatekeeperResult(
                passed=False,
                checks={},
                failures=[f"Runner error: {result.error}"],
                score=0.0
            )
            return EvaluationResult(
                role=role,
                model=result.model,
                backend=runner.name,
                gatekeeper=gate_result,
                judge=None,
                response="",
                execution_time=execution_time
            )

        # Gatekeeper validation
        gate_result = self.gatekeeper.validate(result.content, requirements)

        if self.debug:
            print(f"[Evaluator] Gatekeeper: {gate_result}")

        # Judge evaluation (if gatekeeper passed or we want to judge anyway)
        judge_result = None
        if gate_result.passed or not skip_judge_on_gate_fail:
            judge_result = self.judge.evaluate(result.content, role, benchmark)
            if self.debug:
                print(f"[Evaluator] Judge: {judge_result}")

        eval_result = EvaluationResult(
            role=role,
            model=result.model,
            backend=runner.name,
            gatekeeper=gate_result,
            judge=judge_result,
            response=result.content,
            execution_time=execution_time
        )

        # Store the score
        self.score_store.record(eval_result)

        return eval_result

    def benchmark_all_roles(
        self,
        runner: CLIRunner,
        roles: Optional[List[str]] = None
    ) -> Dict[str, EvaluationResult]:
        """
        Benchmark a runner across all (or specified) roles.

        Args:
            runner: The runner to benchmark
            roles: Specific roles to test (default: all)

        Returns:
            Dict mapping role → EvaluationResult
        """
        roles = roles or list(ROLE_BENCHMARKS.keys())
        results = {}

        for role in roles:
            if self.debug:
                print(f"\n{'='*50}")
                print(f"Benchmarking {runner.name} on {role}")
                print('='*50)

            results[role] = self.evaluate(runner, role)

        return results

    def compare_runners(
        self,
        runners: List[CLIRunner],
        role: str
    ) -> List[EvaluationResult]:
        """
        Compare multiple runners on a single role.

        Args:
            runners: List of runners to compare
            role: Role to evaluate

        Returns:
            List of EvaluationResult, sorted by score (best first)
        """
        results = [self.evaluate(r, role) for r in runners]
        return sorted(results, key=lambda r: r.final_score, reverse=True)

    def generate_report(self) -> str:
        """Generate a summary report of all benchmark scores."""
        lines = [
            "# Model Evaluation Report",
            f"Generated: {datetime.now().isoformat()}",
            "",
            "## Leaderboard by Role",
            ""
        ]

        for role in ROLE_BENCHMARKS.keys():
            leaderboard = self.score_store.get_leaderboard(role)
            lines.append(f"### {role.title()}")

            if not leaderboard:
                lines.append("No benchmarks recorded yet.")
            else:
                lines.append("| Rank | Backend | Model | Score | Time |")
                lines.append("|------|---------|-------|-------|------|")
                for i, entry in enumerate(leaderboard[:5], 1):
                    lines.append(
                        f"| {i} | {entry['backend']} | {entry['model'][:20]} | "
                        f"{entry['score']:.1f}/10 | {entry['execution_time']:.1f}s |"
                    )
            lines.append("")

        return "\n".join(lines)


# =============================================================================
# Convenience Functions
# =============================================================================

def quick_evaluate(
    runner: CLIRunner,
    role: str,
    judge_runner: Optional[CLIRunner] = None
) -> EvaluationResult:
    """
    Quick evaluation of a runner on a role.

    Args:
        runner: Runner to evaluate
        role: Role to test
        judge_runner: Runner for judging (defaults to same runner)

    Returns:
        EvaluationResult
    """
    evaluator = ModelEvaluator(
        judge_runner=judge_runner or runner,
        debug=True
    )
    return evaluator.evaluate(runner, role)


def list_roles() -> List[str]:
    """List all available benchmark roles."""
    return list(ROLE_BENCHMARKS.keys())


def get_role_benchmark(role: str) -> Dict[str, Any]:
    """Get the benchmark definition for a role."""
    return ROLE_BENCHMARKS.get(role, {})


# =============================================================================
# CLI Entry Point
# =============================================================================

if __name__ == "__main__":
    print("Model Evaluator - Meritocratic Model Selection System")
    print("=" * 55)
    print()
    print("Available roles for benchmarking:")
    for role in ROLE_BENCHMARKS:
        reqs = ROLE_BENCHMARKS[role]["requirements"]
        print(f"  - {role}: min {reqs.get('min_length', 0)} chars, {len(reqs)} requirements")
    print()
    print("Usage:")
    print("""
    from model_evaluator import ModelEvaluator, quick_evaluate
    from runners import ClaudeRunner, GroqRunner

    # Quick evaluation
    result = quick_evaluate(GroqRunner(), "executor")
    print(f"Score: {result.final_score}/10")

    # Full benchmark
    evaluator = ModelEvaluator(judge_runner=ClaudeRunner())
    results = evaluator.benchmark_all_roles(GroqRunner())

    # Get best model for a role
    backend, model = evaluator.selector.select("architect")
    print(f"Best architect: {backend}:{model}")
    """)
