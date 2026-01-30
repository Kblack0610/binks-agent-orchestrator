---
name: self-heal
description: Analyze workflow failures and propose/apply improvements
compat:
  binks_agent: true
io:
  input: "failure notification or manual trigger"
  output: "improvement proposal + inbox notification"
tools:
  - self_healing_mcp::*
  - inbox_mcp::write_inbox
  - memory_mcp::learn
---

# Self-Healing Workflow Agent

This agent analyzes workflow failures, detects patterns, and proposes automated improvements to system health.

## Workflow Overview

The self-healing workflow follows a 5-step improvement lifecycle:
1. **Detect Patterns** - Identify recurring failures
2. **Propose Improvements** - Generate fixes for detected patterns
3. **User Review** - Present proposals for approval
4. **Apply Approved** - Implement approved fixes
5. **Monitor Impact** - Verify improvement effectiveness

## Step 1: Detect Patterns

Analyze recent run history to identify recurring failure patterns.

**Actions:**
- Call `detect_failure_patterns(since="-7d", min_occurrences=3)`
- If no patterns found: Report "System healthy, no patterns detected"
- If patterns found: Proceed to Step 2

**Output:**
```json
{
  "patterns": [
    {
      "pattern_id": "abc123",
      "error_type": "Timeout",
      "tool_name": "mcp__kubernetes__pods_list",
      "occurrences": 5,
      "affected_runs": ["run-1", "run-2", ...],
      "confidence": 0.87
    }
  ]
}
```

## Step 2: Propose Improvements

For each detected pattern, generate improvement proposal.

**Actions:**
- For each pattern: Call `propose_improvement(pattern_id="{id}")`
- Rank proposals by priority (high/medium/low)
- Write summary to inbox-mcp using structured digest format

**Digest Format:**
```markdown
# Self-Healing Analysis - [Date]

## 🔴 High Priority Patterns

### Pattern abc123: Kubernetes Timeout
- **Error:** Timeout on mcp__kubernetes__pods_list
- **Occurrences:** 5 in last 7 days
- **Affected runs:** run-abc, run-def, run-ghi
- **Proposed fix:** Increase timeout threshold from 60s to 90s
- **Expected impact:** 40% reduction in timeout errors
- **Actions:** `/self-heal test abc123` or `/self-heal apply abc123`

## 🟡 Medium Priority Patterns
[Additional patterns...]

## Summary
Total patterns detected: 3
High priority: 1
Medium priority: 2
```

**Inbox Notification:**
```bash
inbox_mcp::write_inbox(
  message="Detected 3 failure patterns. View details in analysis digest.",
  priority="high",
  tags=["self-heal", "patterns"],
  source="self-heal"
)
```

## Step 3: User Review (Manual Mode)

Present proposals for user decision.

**User Options:**
- **Approve:** Proceed to Step 4 (apply improvement)
- **Test:** Run simulation first via `test_improvement(mode="simulation")`
- **Reject:** Mark proposal as rejected

**Testing Command:**
```bash
# Simulate improvement against historical data
test_improvement(
  improvement_id="abc123",
  test_mode="simulation"
)
```

**Test Output:**
```json
{
  "improvement_id": "abc123",
  "test_mode": "simulation",
  "simulated_impact": 0.42,
  "confidence": 0.91,
  "recommendation": "Safe to apply - high confidence, positive impact"
}
```

## Step 4: Apply Approved

Implement the approved improvement.

**Actions:**
1. Execute the fix (update config, modify code, adjust settings)
2. Record application:
   ```bash
   apply_improvement(
     improvement_id="abc123",
     changes_made="Updated timeout_ms in config: 60000 → 90000",
     commit_hash="optional-git-hash"
   )
   ```
3. Notify via inbox:
   ```markdown
   ## Applied Improvement abc123

   **Description:** Increase kubernetes timeout threshold
   **Changes:** Updated ~/.binks/config.toml timeout_ms: 60000 → 90000
   **Commit:** 1a2b3c4 (if applicable)

   Monitoring for 7 days to verify impact.
   ```

**Memory Integration:**
Optional - store learnings about the fix:
```bash
memory_mcp::learn(
  entity="improvement:abc123",
  entity_type="improvement",
  facts=[
    {key: "error_type", value: "Timeout"},
    {key: "tool", value: "kubernetes"},
    {key: "fix", value: "Increased timeout threshold"},
    {key: "applied_at", value: "2026-01-30"}
  ]
)
```

## Step 5: Monitor Impact

After 7 days, verify the improvement's actual impact.

**Actions:**
- Wait 7 days (or user-specified measurement window)
- Call `verify_improvement(improvement_id="abc123", measurement_window_days=7)`
- Compare metrics: success rate before vs after
- Report findings via inbox

**Verification Output:**
```json
{
  "improvement_id": "abc123",
  "expected_impact": "40% reduction in timeout errors",
  "actual_impact": 0.45,
  "success_rate_before": 0.82,
  "success_rate_after": 0.94,
  "runs_analyzed": 127,
  "recommendation": "Keep - improvement exceeded expectations"
}
```

**Inbox Notification:**
```markdown
## ✅ Verified Improvement abc123

**Expected:** 40% reduction in timeout errors
**Actual:** 45% reduction (better than expected!)
**Recommendation:** Keep

**Metrics:**
- Success rate before: 82%
- Success rate after: 94%
- Runs analyzed: 127
```

## Trigger Modes

### 1. Manual Trigger
```bash
# Via binks agent
binks self-heal

# Via Claude CLI
claude /self-heal
```

### 2. Error-Triggered (Hook Integration)
Automatically invoked when a run fails with classified error:
- On ToolCallError::Timeout → Detect patterns immediately
- On ToolCallError::ServerCrashed → Analyze crash patterns
- If pattern confidence > 0.8 → Auto-propose fix

### 3. Background Daemon
```bash
# Run analysis every hour
agent self-heal-daemon --interval 1h

# Check for patterns and auto-propose (no auto-apply)
agent self-heal-daemon --auto-detect --no-auto-apply
```

## Error Handling

**Pattern detection fails:**
- Log error to ~/.binks/logs/self-heal.log
- Notify via inbox: "Self-heal analysis failed: [reason]"
- Continue with other patterns if partial failure

**Apply improvement fails:**
- Rollback any partial changes
- Mark improvement as "failed"
- Notify via inbox with error details
- Do not schedule verification

**Verification inconclusive:**
- If sample size < 20 runs: "Need more data, extending window to 14 days"
- If actual impact unclear: "No significant change detected"
- Recommendation: "Monitor for another 7 days" or "Consider rollback"

## Configuration

Settings can be configured in ~/.binks/config.toml:

```toml
[self_heal]
# Pattern detection thresholds
min_occurrences = 3
confidence_threshold = 0.75
lookback_days = 7

# Verification settings
measurement_window_days = 7
min_sample_size = 20

# Auto-apply settings (use with caution)
auto_apply_enabled = false
auto_apply_confidence_threshold = 0.95

# Notification settings
notify_on_detect = true
notify_on_apply = true
notify_on_verify = true
inbox_priority = "high"
```

## Best Practices

1. **Start with testing:** Always test high-impact improvements in simulation mode first
2. **Review before auto-apply:** Keep auto_apply_enabled = false initially
3. **Monitor verification:** Check inbox after 7 days for verification results
4. **Iterate on fixes:** If improvement doesn't work, reject and propose alternative
5. **Document learnings:** Use memory_mcp to track what worked and what didn't

## Example Workflow

```bash
# Manual invocation
$ binks self-heal

> Detecting failure patterns (last 7 days, min 3 occurrences)...
> Found 2 patterns:
>   - abc123: Kubernetes timeout (5 occurrences, confidence: 0.87)
>   - def456: Git connection refused (4 occurrences, confidence: 0.73)
>
> Proposing improvements...
> ✓ Created proposal for abc123
> ✓ Created proposal for def456
>
> Wrote digest to inbox: ~/.notes/inbox/2026-01-30.md
>
> Next steps:
>   - Review proposals in inbox
>   - Test: /self-heal test abc123
>   - Apply: /self-heal apply abc123

# Test improvement
$ binks self-heal test abc123

> Running simulation for improvement abc123...
> Simulated against 50 historical runs
> Expected impact: 42% reduction in timeouts
> Confidence: 91%
> Recommendation: Safe to apply
>
> Apply now? (y/N) y

# Apply improvement
> Applying improvement abc123...
> Updated ~/.binks/config.toml: timeout_ms 60000 → 90000
> Recorded in database
> Notified via inbox
>
> Verification scheduled for 2026-02-06

# (7 days later - automatic verification)
> [Inbox notification]
> ✅ Verified improvement abc123
> Actual impact: 45% reduction (exceeded expectations!)
> Success rate: 82% → 94%
> Recommendation: Keep
```
