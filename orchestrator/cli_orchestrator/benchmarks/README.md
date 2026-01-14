# Workflow Benchmarks

Comprehensive benchmark suite for testing and comparing AI model performance across different workflow types.

## Quick Start

```bash
# View available test projects
python test_projects.py --stats

# Run benchmarks with Claude
python run_workflow_tests.py --backend claude --workflow simple

# Run benchmarks with Gemini
python run_workflow_tests.py --backend gemini --workflow simple

# Run a specific test
python run_workflow_tests.py simple_01 --backend claude

# Dry run (triage only, no execution)
python run_workflow_tests.py --dry-run --workflow simple
```

## Test Projects

30 test projects across 5 workflow types and 3 difficulty levels:

### By Workflow
| Workflow | Count | Description |
|----------|-------|-------------|
| quick | 4 | Direct answers, no coding needed |
| simple | 8 | Single function implementations |
| standard | 8 | Multi-component tasks |
| full | 5 | System design and architecture |
| debug | 5 | Bug fixing and troubleshooting |

### By Difficulty
| Difficulty | Count |
|------------|-------|
| beginner | 13 |
| intermediate | 11 |
| advanced | 6 |

### Filtering
```bash
# View only simple workflow tests
python test_projects.py simple

# View only intermediate difficulty
python test_projects.py intermediate

# Combine filters
python test_projects.py simple beginner
```

## Benchmark Results

Results are saved to dated files:
- `YYYY-MM-DD_workflow_results.json` - Raw JSON results
- `YYYY-MM-DD_workflow_summary.md` - Human-readable summary

### Latest Results (2025-12-12)

| Model | Avg Score | Reliability | Avg Time |
|-------|-----------|-------------|----------|
| Claude | 8.9/10 | 100% | ~8.0s |
| Gemini | 8.5/10 | 88% | ~5.5s |

See `comparison/2025-12-12_comparison_report.md` for detailed analysis.

## Supported Backends

| Backend | Runner | Description |
|---------|--------|-------------|
| claude | ClaudeRunner | Claude Code CLI |
| gemini | GeminiRunner | Gemini CLI |
| groq | GroqRunner | Groq API |
| auto | config.py | Role-based model selection |

## Scoring System

Each response is scored using two components:

### 1. Gatekeeper (Heuristic)
Fast check for basic quality:
- Expected keywords present
- Minimum response length
- Code block formatting

### 2. Judge (LLM Evaluation)
Detailed quality assessment:
- Correctness
- Completeness
- Code quality
- Documentation

**Final Score** = 0.7 * Judge + 0.3 * Gatekeeper

## Files

```
benchmarks/
├── README.md                    # This file
├── test_projects.py            # 30 test project definitions
├── run_workflow_tests.py       # Benchmark runner
├── MODEL_SELECTION_ROADMAP.md  # Future roadmap
├── comparison/                  # Model comparison results
│   ├── 2025-12-12_claude_results.json
│   ├── 2025-12-12_gemini_results.json
│   └── 2025-12-12_comparison_report.md
└── YYYY-MM-DD_workflow_*.json/md  # Daily results
```

## Adding New Tests

Edit `test_projects.py`:

```python
{
    "id": "simple_09",
    "name": "My New Test",
    "task": "Write a Python function that...",
    "expected_workflow": "simple",
    "complexity": "simple",
    "category": "code",
    "difficulty": "beginner",
    "expected_answer_contains": ["def", "return"],
}
```

## Historical Results

| Date | Backend | Tests | Avg Score | Triage | Notes |
|------|---------|-------|-----------|--------|-------|
| 2025-12-12 | Claude | 8 | 8.9/10 | 75% | All passed gatekeeper |
| 2025-12-12 | Gemini | 8 | 8.5/10 | 88% | 1 empty response |
| 2025-12-12 | Mixed | 2 | 9.7/10 | 100% | Initial baseline |

## Roadmap

See `MODEL_SELECTION_ROADMAP.md` for the meritocratic model selection roadmap:
- Phase 1: Data Collection
- Phase 2: Performance Tracking
- Phase 3: Model Selection Logic
- Phase 4: Fallback & Retry
- Phase 5: Cost Optimization
- Phase 6: Specialized Agents
- Phase 7: Continuous Learning
