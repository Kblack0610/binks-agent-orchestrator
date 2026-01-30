# Self-Healing System

## Overview

The self-healing system automatically detects recurring workflow failures, proposes improvements, and can apply fixes to increase system reliability. It consists of three main components:

1. **self-healing-mcp**: MCP server providing analysis and improvement tools
2. **agent self-heal command**: CLI interface for manual workflow control
3. **inbox integration**: User notifications for detected patterns and improvements

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Agent CLI                                │
│  (agent self-heal detect|show|test|apply|verify|dashboard)  │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│              self-healing-mcp Server                         │
│  • Pattern Detection      • Health Analysis                  │
│  • Improvement Proposals  • Metrics Computation              │
│  • Fix Application        • Verification                     │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                Run History Database                          │
│  ~/.binks/conversations.db                                   │
│  • runs table          • run_events table                    │
│  • improvements table  • tool metrics                        │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Pattern Detection**: Query run_events table for recurring errors
2. **Improvement Proposal**: Generate fix strategy based on error patterns
3. **Notification**: Write summary to inbox-mcp for user review
4. **Application** (future): Apply approved improvements to config/code
5. **Verification** (future): Measure actual impact vs expected

## MCP Tools

### Analysis Tools

#### `detect_failure_patterns`
Finds recurring errors across workflow runs.

**Parameters:**
- `since` (string): Time window (e.g., "-7d", "-24h")
- `min_occurrences` (number): Minimum times error must occur (default: 3)
- `confidence_threshold` (number): Correlation confidence 0.0-1.0 (default: 0.75)

**Returns:**
```json
[
  {
    "id": "tool_error:list_dir",
    "error_type": "tool_error",
    "tool_name": "list_dir",
    "occurrences": 5,
    "correlation_score": 0.8,
    "affected_runs": ["run-abc", "run-def"],
    "suggested_fix": "Add file existence checks before list_dir calls"
  }
]
```

#### `analyze_run_health`
Computes health score for a single workflow run.

**Parameters:**
- `run_id` (string): Run ID to analyze

**Returns:**
```json
{
  "run_id": "run-abc123",
  "health_score": 85.5,
  "issues": [
    {
      "severity": "medium",
      "description": "Tool timeout on kubernetes_pods_list",
      "recommendation": "Increase timeout threshold"
    }
  ]
}
```

#### `compute_agent_metrics`
Calculates success rates per agent type.

**Parameters:**
- `since` (string): Time window (default: "-7d")

**Returns:**
```json
{
  "agents": [
    {
      "name": "planner",
      "success_rate": 0.92,
      "total_runs": 50,
      "avg_duration_seconds": 45.2
    }
  ]
}
```

#### `compute_tool_reliability`
Measures reliability scores per MCP tool.

**Parameters:**
- `since` (string): Time window (default: "-7d")

**Returns:**
```json
{
  "tools": [
    {
      "name": "kubernetes_pods_list",
      "success_rate": 0.85,
      "total_calls": 200,
      "timeout_rate": 0.10,
      "error_rate": 0.05
    }
  ]
}
```

### Improvement Lifecycle Tools

#### `propose_improvement`
Generates fix proposal for a detected pattern.

**Parameters:**
- `pattern_id` (string): Pattern ID from detect_failure_patterns

**Returns:**
```json
{
  "improvement_id": "unknown",
  "pattern_id": "tool_error:list_dir",
  "description": "Add file existence checks before list_dir operations",
  "expected_impact": "30-40% reduction in list_dir errors",
  "changes_required": [
    "Update planner prompt to verify paths",
    "Add filesystem check before list_dir calls"
  ],
  "risk_level": "low"
}
```

#### `test_improvement`
Simulates improvement against historical data.

**Parameters:**
- `improvement_id` (string): Improvement to test
- `mode` (string): "simulation" | "canary" | "shadow"

**Returns:**
```json
{
  "improvement_id": "imp-123",
  "test_mode": "simulation",
  "simulated_success_rate": 0.92,
  "baseline_success_rate": 0.85,
  "expected_improvement": "+7%",
  "recommendation": "safe_to_apply"
}
```

#### `apply_improvement`
Marks improvement as applied with changes made.

**Parameters:**
- `improvement_id` (string): Improvement to apply
- `changes_made` (string): Description of actual changes

**Returns:**
```json
{
  "improvement_id": "imp-123",
  "status": "applied",
  "applied_at": "2026-01-30T12:00:00Z",
  "verification_scheduled": "2026-02-06T12:00:00Z"
}
```

#### `verify_improvement`
Measures actual impact after application.

**Parameters:**
- `improvement_id` (string): Improvement to verify
- `measurement_window_days` (number): Days to analyze (default: 7)

**Returns:**
```json
{
  "improvement_id": "imp-123",
  "expected_impact": "+7% success rate",
  "actual_impact": "+9% success rate",
  "verification_status": "verified",
  "recommendation": "keep",
  "metrics": {
    "before_success_rate": 0.85,
    "after_success_rate": 0.94,
    "runs_analyzed": 127
  }
}
```

### Dashboard Tool

#### `get_health_dashboard`
Provides system-wide health summary.

**Parameters:**
- `detailed` (boolean): Include per-agent breakdown

**Returns:**
```json
{
  "overall_health_score": 88.5,
  "trend": "improving",
  "success_rate": 0.92,
  "recent_improvements": [
    {
      "id": "imp-123",
      "description": "Increased timeout thresholds",
      "impact": "+7% success rate"
    }
  ]
}
```

## CLI Commands

### `agent self-heal detect`
Detect failure patterns and propose improvements.

**Options:**
- `--since <DURATION>`: Time window (default: "-7d")
- `--min-occurrences <N>`: Minimum occurrences (default: 3)
- `--confidence <FLOAT>`: Confidence threshold 0.0-1.0 (default: 0.75)

**Example:**
```bash
agent self-heal detect --since -24h --min-occurrences 2
```

**Output:**
```
🔍 Detecting failure patterns (last -24h, min 2 occurrences, confidence >= 75%)...

📊 Found 4 pattern(s):

  Pattern tool_error:list_dir
    Error: tool_error on list_dir
    Occurrences: 5
    Correlation: 80%

  Pattern Unknown:execute_command
    Error: Unknown on execute_command
    Occurrences: 3
    Correlation: 50%

💡 Generating improvement proposals...

  ✓ Proposal unknown for pattern tool_error:list_dir
  ✓ Proposal unknown for pattern Unknown:execute_command

📬 Writing summary to inbox...

📋 Next steps:
  - Test: agent self-heal test <improvement-id>
  - Apply: agent self-heal apply <improvement-id>
  - View: agent self-heal show <pattern-id>
```

### `agent self-heal show <pattern-id>`
Show detailed information about a detected pattern.

**Status:** ⚠️ Not yet implemented

**Will show:**
- Error type and affected tool
- List of affected runs with timestamps
- Context similarity analysis
- Suggested fix strategy

### `agent self-heal test <improvement-id>`
Test improvement in simulation mode.

**Status:** ⚠️ Not yet implemented

**Will perform:**
- Run simulation against historical data
- Show expected vs simulated impact
- Provide recommendation (safe/risky/reject)

### `agent self-heal apply <improvement-id>`
Apply an approved improvement.

**Options:**
- `--yes`: Skip confirmation prompt

**Status:** ⚠️ Not yet implemented

**Will perform:**
- Update config/code as specified
- Record application in database
- Write notification to inbox
- Schedule verification after 7 days

### `agent self-heal verify <improvement-id>`
Verify improvement's actual impact.

**Options:**
- `--window-days <N>`: Measurement window (default: 7)

**Status:** ⚠️ Not yet implemented

**Will perform:**
- Compare metrics before vs after
- Calculate actual impact percentage
- Recommend keep/rollback/extend monitoring
- Write verification report to inbox

### `agent self-heal dashboard`
Show system health dashboard.

**Options:**
- `--detailed`: Include per-agent metrics

**Status:** ⚠️ Not yet implemented

**Will show:**
- Overall health score (0-100)
- Success rate trends (improving/degrading/stable)
- Per-agent metrics (if --detailed)
- Per-tool reliability scores
- Recent improvements and their impact

### `agent self-heal improvements`
List improvements.

**Options:**
- `--status <STATUS>`: Filter by status (proposed/applied/verified/rejected)
- `--limit <N>`: Maximum results (default: 20)

**Status:** ⚠️ Not yet implemented

**Will show:**
- Improvement ID
- Status (proposed/applied/verified/rejected)
- Description
- Expected vs actual impact
- Applied date (if applicable)

## Inbox Notifications

The self-healing system writes structured notifications to inbox-mcp for user awareness.

### Pattern Detection Notification

```markdown
## 2026-01-30 02:09:03 [self-heal] #self-heal #patterns *[HIGH]*
Detected 4 failure pattern(s). View details: agent self-heal improvements
```

### Improvement Applied Notification (Future)

```markdown
## 2026-01-30 15:00:00 [self-heal] #improvement #applied *[NORMAL]*

Applied improvement imp-123:
Description: Increase timeout for kubernetes tools
Changes: Updated ~/.binks/config.toml timeout_ms: 60000 → 90000
Commit: 1a2b3c4

Monitoring for 7 days to verify impact.
```

### Verification Result Notification (Future)

```markdown
## 2026-02-06 15:00:00 [self-heal] #improvement #verified *[NORMAL]*

Verified improvement imp-123:
Expected: 40% reduction in timeout errors
Actual: 45% reduction (better than expected!)
Recommendation: Keep

Metrics:
- Success rate before: 82%
- Success rate after: 94%
- Runs analyzed: 127
```

## Error Pattern Detection

### Algorithm

1. **Query Error Events**: Extract tool_complete events where is_error=true from run_events table
2. **Group by Signature**: Group by (error_type, tool_name) to identify patterns
3. **Filter by Threshold**: Keep patterns with occurrences >= min_occurrences
4. **Compute Correlation**: Calculate context similarity using Jaccard index
5. **Generate Fix**: Map error type to improvement strategy

### Supported Error Types

| Error Type | Example | Fix Strategy |
|------------|---------|--------------|
| `tool_error` | File not found | Add existence checks, glob fallback |
| `timeout` | Long-running operation | Increase timeouts, add parallelization |
| `connection_refused` | Server unavailable | Health checks, auto-restart, retries |
| `server_crashed` | MCP server failure | Circuit breaker, resource limits |
| `permission_denied` | Access denied | Update permissions table |
| `Unknown` | NULL error_type | Analyze context, generic improvements |

### Special Case: NULL Error Handling

Some tool errors may have NULL error_type in the database. The system handles this with:

```sql
-- Match both NULL and 'Unknown' error_type
WHERE json_extract(event_data, '$.error_type') IS NULL
   OR json_extract(event_data, '$.error_type') = 'Unknown'
```

Pattern IDs use "Unknown" as the error_type string: `Unknown:tool_name`

## Health Scoring

### Formula

```
health_score = weighted_sum([
    (success_rate, 0.5),           # 50% - most critical
    (avg_duration_normalized, 0.2), # 20% - performance
    (tool_reliability, 0.2),        # 20% - stability
    (resource_efficiency, 0.1)      # 10% - optimization
])
```

### Trend Detection

- **Recent period**: Last 7 days
- **Historical period**: 30 days before recent
- **Improving**: recent > historical + 5%
- **Degrading**: recent < historical - 5%
- **Stable**: within ±5%

## Database Schema

### improvements table

```sql
CREATE TABLE IF NOT EXISTS improvements (
    id TEXT PRIMARY KEY,
    pattern_id TEXT NOT NULL,
    description TEXT NOT NULL,
    expected_impact TEXT,
    changes_made TEXT,
    status TEXT NOT NULL,  -- proposed, applied, verified, rejected
    proposed_at TEXT NOT NULL,
    applied_at TEXT,
    verified_at TEXT,
    actual_impact TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**Note:** Currently improvements are proposed but not persisted to the database. Database persistence will be implemented in a future phase.

## Configuration

### MCP Server Configuration

Add to `.mcp.json`:

```json
{
  "mcpServers": {
    "self-healing": {
      "command": "/path/to/self-healing-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      },
      "tier": 2
    }
  }
}
```

### Environment Variables

- `RUST_LOG`: Logging level (error, warn, info, debug, trace)
- Database path is always `~/.binks/conversations.db` (hardcoded)

## Usage Examples

### Manual Pattern Detection

```bash
# Detect patterns in last 7 days (default)
agent self-heal detect

# Detect patterns in last 24 hours with stricter threshold
agent self-heal detect --since -24h --min-occurrences 5

# Detect patterns with lower confidence threshold
agent self-heal detect --confidence 0.5
```

### View Pattern Details

```bash
# Show details for specific pattern
agent self-heal show tool_error:list_dir
```

### Test and Apply Improvements

```bash
# Test improvement in simulation mode
agent self-heal test imp-123

# Apply improvement (with confirmation)
agent self-heal apply imp-123

# Apply improvement (skip confirmation)
agent self-heal apply imp-123 --yes
```

### Monitor Impact

```bash
# Verify improvement after 7 days (default)
agent self-heal verify imp-123

# Custom measurement window
agent self-heal verify imp-123 --window-days 14
```

### Health Dashboard

```bash
# Basic health overview
agent self-heal dashboard

# Detailed per-agent metrics
agent self-heal dashboard --detailed
```

### List Improvements

```bash
# List all improvements
agent self-heal improvements

# Filter by status
agent self-heal improvements --status applied

# Limit results
agent self-heal improvements --limit 10
```

## Implementation Status

### ✅ Completed

- [x] self-healing-mcp server scaffold
- [x] 9 MCP tools (all handlers implemented)
- [x] Pattern detection algorithm
- [x] Improvement proposal generation
- [x] Inbox notification integration
- [x] CLI command structure
- [x] `detect` subcommand implementation
- [x] NULL error_type SQL handling
- [x] Deterministic pattern IDs
- [x] End-to-end workflow testing

### 🚧 Stub Implementations (Future Work)

- [ ] `show` subcommand - Pattern detail view
- [ ] `test` subcommand - Simulation testing
- [ ] `apply` subcommand - Automated fix application
- [ ] `verify` subcommand - Impact measurement
- [ ] `dashboard` subcommand - Health visualization
- [ ] `improvements` subcommand - Improvement listing
- [ ] Database persistence of improvements
- [ ] Automated fix application logic
- [ ] Statistical confidence testing
- [ ] Circuit breaker for risky fixes
- [ ] Git commit integration for code changes
- [ ] Scheduled verification (cron/daemon)

## Future Enhancements

### Trigger Modes

**1. Manual Trigger** (Current)
```bash
agent self-heal detect
```

**2. Error-Triggered (Planned)**
- Hook integration: detect patterns immediately after run failure
- Automatic analysis on workflow errors
- Instant notification via inbox

**3. Background Daemon (Planned)**
```bash
agent self-heal-daemon --interval 1h
```
- Continuous monitoring
- Hourly/daily pattern detection
- Auto-proposal for high-confidence fixes

### Advanced Features

- **Auto-Apply with Approval**: Automatically apply low-risk improvements
- **Canary Testing**: Test fixes on small subset before full rollout
- **Shadow Mode**: Run new logic alongside old for comparison
- **Rollback Capability**: Automatic revert if success rate drops
- **A/B Testing**: Compare different fix strategies
- **Learning from Success**: Detect improving patterns to replicate
- **Cross-Project Learning**: Share improvements across repositories

## Troubleshooting

### No Patterns Detected

**Possible causes:**
- Not enough recent failures (< min_occurrences)
- Time window too narrow (use --since -30d)
- Errors not classified correctly in run_events
- Correlation scores below threshold

**Solutions:**
- Lower `--min-occurrences` threshold
- Expand time window with `--since`
- Lower `--confidence` threshold
- Check run_events table for error data

### MCP Connection Failed

**Error:**
```
Error: No .mcp.json found - self-healing-mcp tools required
```

**Solution:**
- Ensure `.mcp.json` exists in working directory
- Verify self-healing-mcp is in configuration
- Check that binary path is correct
- Test MCP server directly: `agent tools --server self-healing`

### Database Permission Errors

**Error:**
```
Error: Failed to open database: unable to open database file
```

**Solution:**
- Ensure `~/.binks/` directory exists
- Check file permissions on conversations.db
- Verify not running multiple instances concurrently
- Check disk space available

## References

- Implementation plan: `.claude/plans/snoopy-sprouting-dewdrop.md`
- MCP server code: `mcps/self-healing-mcp/src/`
- CLI handler: `agent/src/handlers/selfheal.rs`
- Database schema: `agent/src/db/schema.rs`
- Run events: `agent/src/agent/events.rs`
