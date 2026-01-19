# Scratchpad MCP

Structured reasoning and thinking chain tracker.

## Overview

Scratchpad MCP provides a lightweight system for recording and managing reasoning chains. Unlike full sequential thinking systems, it's designed to be simple and session-scoped.

**Use cases:**
- Complex debugging (track hypotheses)
- Multi-step planning (record decisions)
- Analysis tasks (track findings)
- Learning (record insights)

## Tools

| Tool | Parameters | Description |
|------|------------|-------------|
| `think` | `thought: string, confidence?: float` | Record reasoning step |
| `revise` | `step_id: int, new_thought: string` | Revise previous step |
| `get_reasoning_chain` | - | Get all thinking steps |
| `summarize_thinking` | - | Get condensed chain |
| `clear_scratchpad` | - | Reset scratchpad |

## Configuration

```json
{
  "mcpServers": {
    "scratchpad": {
      "command": "./mcps/scratchpad-mcp/target/release/scratchpad-mcp"
    }
  }
}
```

## Data Model

```rust
struct Scratchpad {
    entries: Vec<ThinkingStep>,
}

struct ThinkingStep {
    step_num: u32,
    thought: String,
    confidence: f32,      // 0.0 - 1.0
    revises: Option<u32>, // if this revises a previous step
    timestamp: DateTime<Utc>,
}
```

## Usage Examples

### Recording analysis

```json
// Investigating a bug
{ "tool": "think", "arguments": {
    "thought": "Error happens on line 45 of auth.rs",
    "confidence": 0.9
}}
// Returns: { "step_id": 1 }

{ "tool": "think", "arguments": {
    "thought": "Likely a race condition in token refresh",
    "confidence": 0.7
}}
// Returns: { "step_id": 2 }

{ "tool": "think", "arguments": {
    "thought": "Confirmed: multiple threads accessing shared state",
    "confidence": 0.95
}}
// Returns: { "step_id": 3 }
```

### Revising thinking

```json
// Realized earlier hypothesis was wrong
{ "tool": "revise", "arguments": {
    "step_id": 2,
    "new_thought": "Actually a deadlock, not race condition"
}}
// Returns: { "step_id": 4, "revises": 2 }
```

### Getting the chain

```json
{ "tool": "get_reasoning_chain", "arguments": {} }
// Returns: [
//   { "step_num": 1, "thought": "Error happens...", "confidence": 0.9 },
//   { "step_num": 2, "thought": "Likely a race...", "confidence": 0.7 },
//   { "step_num": 3, "thought": "Confirmed...", "confidence": 0.95 },
//   { "step_num": 4, "thought": "Actually a deadlock...", "confidence": 0.8, "revises": 2 }
// ]
```

### Summarizing

```json
{ "tool": "summarize_thinking", "arguments": {} }
// Returns: "Bug investigation: Found error at auth.rs:45. Initially suspected
//           race condition, revised to deadlock. Confirmed multiple threads
//           accessing shared state."
```

## vs Sequential Thinking MCP

| Feature | Scratchpad | Sequential Thinking |
|---------|------------|---------------------|
| Complexity | Simple | Complex |
| Branching | No | Yes |
| Backtracking | Revise only | Full tree |
| Persistence | Session | Configurable |
| Use case | Quick analysis | Deep reasoning |

**Use Scratchpad for:** Most tasks, quick analysis, debugging
**Use Sequential Thinking for:** Complex multi-path exploration

## Building

```bash
cd mcps/scratchpad-mcp
cargo build --release
```

## Dependencies

- `rmcp` - MCP SDK
- `tokio` - Async runtime
- `serde` - Serialization
- `chrono` - Timestamps
