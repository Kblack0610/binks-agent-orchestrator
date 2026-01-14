# Model Comparison Report: Claude vs Gemini

**Date**: 2025-12-12
**Workflow**: Simple (executor-only tasks)
**Tests**: 8 beginner/intermediate coding tasks

## Executive Summary

| Metric | Claude | Gemini | Winner |
|--------|--------|--------|--------|
| **Average Score** | 8.9/10 | 8.5/10 | Claude |
| **Triage Accuracy** | 75% (6/8) | 88% (7/8) | Gemini |
| **Passed Gatekeeper** | 8/8 (100%) | 7/8 (88%) | Claude |
| **Avg Execution Time** | ~8.0s | ~5.5s | Gemini |
| **Judge Scores (valid)** | 8.3/10 | 9.5/10 | Gemini |

### Key Findings

1. **Claude is more reliable** - 100% passed gatekeeper vs 88% for Gemini
2. **Gemini is faster** - ~30% faster execution times
3. **Gemini produces higher quality code** when it works - higher judge scores
4. **Claude over-triages** - classifies simple tasks as more complex (standard)
5. **Gemini has parsing issues** - triage sometimes fails to extract workflow

## Detailed Results by Test

### Test-by-Test Comparison

| Test | Claude Score | Gemini Score | Claude Time | Gemini Time |
|------|--------------|--------------|-------------|-------------|
| simple_01: String Reverse | 9.5 | 10.0 | 7.6s | 5.2s |
| simple_02: Fizzbuzz | 9.5 | 9.8 | 8.0s | 5.4s |
| simple_03: List Classifier | 9.5 | 10.0 | 7.4s | 5.1s |
| simple_04: Palindrome | 9.5 | 9.8 | 8.3s | 6.0s |
| simple_05: Binary Search | 9.5 | 9.8 | 7.6s | 5.1s |
| simple_06: Merge Lists | 9.5 | 9.5 | 9.0s | 5.0s |
| simple_07: Email Validator | 9.1 | 9.1 | 8.1s | 6.0s |
| simple_08: Rate Limiter | 5.3 | 0.0 | 71.2s | 8.7s |

### Analysis

#### Claude Strengths
- **Reliability**: Never produced empty responses
- **Consistency**: All valid tests scored 9.1-9.5
- **Robustness**: Works even when mis-triaged

#### Claude Weaknesses
- **Over-engineering**: Classifies simple tasks as standard (triggers full workflow)
- **Slower**: ~30% slower than Gemini
- **Verbose prompts**: Sometimes asks for permission in headless mode (fixed)

#### Gemini Strengths
- **Speed**: 30-40% faster execution
- **Quality**: When it works, produces cleaner code with better documentation
- **Triage**: More accurate workflow classification

#### Gemini Weaknesses
- **Reliability**: Occasional empty responses (Rate Limiter test)
- **Parsing**: Triage output sometimes unparseable
- **Quota**: Specific models (gemini-2.5-pro) hit quota limits

## Triage Analysis

### Claude Triage Decisions
```
simple_01: quick    (WRONG - expected simple)
simple_02: simple   (CORRECT)
simple_03: simple   (CORRECT)
simple_04: simple   (CORRECT)
simple_05: simple   (CORRECT)
simple_06: simple   (CORRECT)
simple_07: simple   (CORRECT)
simple_08: standard (WRONG - expected simple)
```

### Gemini Triage Decisions
```
simple_01: simple   (CORRECT)
simple_02: simple   (CORRECT)
simple_03: simple   (CORRECT)
simple_04: simple   (CORRECT)
simple_05: simple   (CORRECT)
simple_06: simple   (CORRECT)
simple_07: simple   (CORRECT)
simple_08: UNKNOWN  (PARSE FAILED)
```

## Recommendations

### For Production Use
1. **Use Claude for reliability** when correctness is critical
2. **Use Gemini for speed** when quality can be sacrificed for throughput
3. **Consider hybrid approach**: Gemini for executor, Claude for judge

### For Model Selection System
1. **Track per-task metrics** to learn which model performs best per category
2. **Implement fallback logic**: If Gemini returns empty, retry with Claude
3. **Weight by recency**: Recent performance should influence selection more

## Code Quality Examples

### Claude Output (simple_01)
```python
def reverse_string(s: str) -> str:
    return s[::-1]
```
- Concise, correct, typed
- No documentation

### Gemini Output (simple_01)
```python
def reverse_string(s: str) -> str:
    """
    Reverses a given string.

    Args:
        s: The input string to be reversed.

    Returns:
        The reversed string.
    """
    return s[::-1]
```
- Well-documented with docstring
- Follows Python conventions

## Test Suite Statistics

```
Total projects: 30
By workflow: debug=5, full=5, quick=4, simple=8, standard=8
By difficulty: advanced=6, beginner=13, intermediate=11
By category: code=15, debug=5, system=5, other=5
```

## Next Steps

1. Run full benchmark suite (all 30 tests) with both models
2. Implement meritocratic model selection based on these results
3. Add more test cases for edge cases identified
4. Track model performance over time for adaptive selection
