# Model Selection Roadmap

## Vision
Build a meritocratic model selection system that automatically chooses the best model for each task based on historical performance data.

## Current State (v0.1)

### What We Have
- **Multiple Runners**: ClaudeRunner, GeminiRunner, GroqRunner, OpenRouterRunner
- **Role-Based Config**: DEFAULT_ROLE_MODELS in config.py
- **Benchmark Framework**: 30 test projects with scoring system
- **Baseline Data**: Claude vs Gemini comparison on simple workflow

### Current Performance
```
Claude: 8.9/10 avg, 100% reliability, ~8s latency
Gemini: 8.5/10 avg, 88% reliability, ~5.5s latency
```

## Roadmap

### Phase 1: Data Collection (Week 1-2)
**Goal**: Establish baseline performance metrics for all models

#### Tasks
- [ ] Run full benchmark suite (30 tests) with all models
- [ ] Store results in structured format (JSON/SQLite)
- [ ] Track metrics: score, latency, reliability, cost
- [ ] Add cost tracking per request

#### Files to Create/Modify
- `cli_orchestrator/benchmarks/benchmark_storage.py` - SQLite storage
- `cli_orchestrator/benchmarks/full_benchmark.py` - Full suite runner
- `cli_orchestrator/config.py` - Add cost per model

### Phase 2: Performance Tracking (Week 2-3)
**Goal**: Track real-world performance during normal usage

#### Tasks
- [ ] Add logging hooks to track every model call
- [ ] Store task → model → result mapping
- [ ] Implement rolling performance calculation
- [ ] Create performance dashboard/report generator

#### Metrics to Track
```python
PerformanceRecord = {
    "timestamp": datetime,
    "model": str,           # claude, gemini, groq
    "role": str,            # executor, triage, judge
    "task_category": str,   # code, debug, system
    "task_difficulty": str, # beginner, intermediate, advanced
    "score": float,         # 0-10
    "latency_ms": int,
    "token_count": int,
    "cost_usd": float,
    "success": bool
}
```

### Phase 3: Model Selection Logic (Week 3-4)
**Goal**: Implement intelligent model selection

#### Strategy: Multi-Armed Bandit with Decay
```python
def select_model(role: str, task: Task) -> str:
    """Select best model using Thompson Sampling."""
    candidates = get_models_for_role(role)

    scores = {}
    for model in candidates:
        # Get historical performance
        history = get_performance_history(model, task.category)

        # Apply exponential decay (recent performance matters more)
        weighted_scores = apply_temporal_decay(history)

        # Thompson Sampling: sample from Beta distribution
        alpha = 1 + sum(weighted_scores)
        beta = 1 + len(weighted_scores) - sum(weighted_scores)
        scores[model] = np.random.beta(alpha, beta)

    return max(scores, key=scores.get)
```

#### Files to Create
- `cli_orchestrator/model_selector.py` - Selection algorithms
- `cli_orchestrator/performance_store.py` - Performance database

### Phase 4: Fallback & Retry (Week 4-5)
**Goal**: Handle failures gracefully

#### Strategy
```python
def execute_with_fallback(task: Task, role: str) -> Result:
    """Execute task with automatic fallback on failure."""
    models = get_ranked_models(role, task.category)

    for model in models:
        try:
            result = execute(model, task)
            if result.is_valid():
                record_success(model, task)
                return result
        except Exception as e:
            record_failure(model, task, e)

    raise AllModelsFailedError(task)
```

### Phase 5: Cost Optimization (Week 5-6)
**Goal**: Balance quality vs cost

#### Cost Tiers
```python
MODEL_COSTS = {
    "claude-opus": 0.015/1k,    # Highest quality, highest cost
    "claude-sonnet": 0.003/1k,  # Good quality, medium cost
    "gemini-pro": 0.00125/1k,   # Good quality, low cost
    "groq-llama": 0.0001/1k,    # Fastest, lowest cost
}
```

#### Selection with Budget
```python
def select_model_with_budget(
    role: str,
    task: Task,
    max_cost: float = 0.10
) -> str:
    """Select best model within budget."""
    candidates = get_models_under_budget(max_cost, task.estimated_tokens)
    return select_best_model(candidates, role, task)
```

### Phase 6: Specialized Agents (Week 6-8)
**Goal**: Train specialized model assignments

#### Task Categories
| Category | Recommended Model | Rationale |
|----------|------------------|-----------|
| Code Generation | Gemini | Higher quality, faster |
| Code Review | Claude | Better reasoning |
| Debugging | Claude | Better analysis |
| Documentation | Gemini | Good formatting |
| Triage | Groq | Fast, cheap |
| Architecture | Claude | Complex reasoning |

### Phase 7: Continuous Learning (Ongoing)
**Goal**: Improve model selection over time

#### Feedback Loop
```
User Task → Select Model → Execute → Score → Update Model Weights
                ↑                                    ↓
                ←────── Next Selection ←─────────────
```

#### A/B Testing
- Randomly assign 10% of tasks to exploration (try different models)
- Use 90% for exploitation (use best known model)

## Implementation Priority

### High Priority (Do First)
1. SQLite storage for benchmark results
2. Cost tracking per model
3. Basic model selection by task category
4. Fallback on failure

### Medium Priority
5. Temporal decay for model scores
6. Performance dashboard
7. Budget-aware selection

### Low Priority (Nice to Have)
8. Thompson Sampling
9. A/B testing framework
10. Real-time adaptation

## Success Metrics

### Primary
- **Quality**: Average score > 9.0/10
- **Reliability**: > 95% success rate
- **Cost**: < $0.05 per task average

### Secondary
- **Latency**: < 10s for simple tasks
- **Efficiency**: Right model selected > 80% of time

## Dependencies

### Required
- SQLite or similar for data storage
- Numpy for statistical calculations
- Logging infrastructure

### Optional
- Grafana/Prometheus for monitoring
- Redis for caching model scores

## Timeline

```
Week 1-2: Phase 1 (Data Collection)
Week 2-3: Phase 2 (Performance Tracking)
Week 3-4: Phase 3 (Model Selection)
Week 4-5: Phase 4 (Fallback & Retry)
Week 5-6: Phase 5 (Cost Optimization)
Week 6-8: Phase 6 (Specialized Agents)
Ongoing:  Phase 7 (Continuous Learning)
```

## Files Structure

```
cli_orchestrator/
├── model_selection/
│   ├── __init__.py
│   ├── selector.py          # Model selection logic
│   ├── performance_store.py # SQLite storage
│   ├── cost_tracker.py      # Cost per model/request
│   └── algorithms/
│       ├── thompson.py      # Thompson Sampling
│       ├── ucb.py           # Upper Confidence Bound
│       └── epsilon.py       # Epsilon-greedy
├── benchmarks/
│   ├── test_projects.py     # Test cases (30 projects)
│   ├── run_workflow_tests.py # Benchmark runner
│   ├── comparison/          # Model comparison results
│   └── MODEL_SELECTION_ROADMAP.md # This file
└── config.py                # DEFAULT_ROLE_MODELS
```

## Getting Started

### Run Benchmarks
```bash
# Run simple workflow comparison
cd benchmarks
python run_workflow_tests.py --backend claude --workflow simple
python run_workflow_tests.py --backend gemini --workflow simple

# View results
cat comparison/2025-12-12_comparison_report.md
```

### Check Test Projects
```bash
python test_projects.py --stats
python test_projects.py simple beginner
```

## References

- [Multi-Armed Bandit Problem](https://en.wikipedia.org/wiki/Multi-armed_bandit)
- [Thompson Sampling](https://en.wikipedia.org/wiki/Thompson_sampling)
- [Explore-Exploit Tradeoff](https://en.wikipedia.org/wiki/Exploration-exploitation_tradeoff)
