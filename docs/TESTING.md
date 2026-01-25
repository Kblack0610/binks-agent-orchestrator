# Testing Philosophy

This document outlines our approach to testing the binks-agent-orchestrator.

## Core Principle: Deterministic Tests Only

**We do not write flaky tests.** Every test must be:

- **Deterministic** - Same input always produces same output
- **Fast** - No network calls, no sleeps, no timeouts as success criteria
- **Meaningful** - Tests behavior that matters, not implementation details

## What We Test

### 1. Tool Call Parsers (High Priority)

Different LLMs output tool calls in different formats. Parser bugs cause silent failures.

**Test categories:**
- JSON format parsing (standard, nested, arrays)
- XML format parsing (various tag styles)
- Malformed input handling (no panics, graceful fallback)
- Edge cases (empty strings, unicode, very long inputs)

**Location:** `agent/src/agent/parsers/`

### 2. Configuration Loading (High Priority)

Config bugs cause runtime failures that are hard to debug.

**Test categories:**
- Environment variable expansion (`${HOME}`, `${USER}`)
- Config hierarchy (project overrides global)
- Missing config fallback to defaults
- Invalid config graceful error messages
- Tier threshold parsing

**Location:** `agent/src/config.rs`

### 3. MCP Tool Routing (Medium Priority)

Wrong tool routing = wrong behavior.

**Test categories:**
- Tier filtering by model size
- Tool name to server mapping
- Tool cache behavior
- Profile-based server selection

**Location:** `agent/src/mcp/`

### 4. Error Boundaries (Medium Priority)

Failures should degrade gracefully, not crash.

**Test categories:**
- Tool execution errors return messages, not panics
- LLM connection failures are reported clearly
- Max iterations prevents infinite loops
- Invalid tool arguments handled

**Location:** Various, inline with modules

## What We Do NOT Test

### Avoid These Anti-Patterns

| Anti-Pattern | Why It's Bad |
|--------------|--------------|
| "LLM returns exactly this string" | Too coupled to model behavior |
| "Agent makes exactly N tool calls" | Implementation detail |
| "Response takes < Xms" | Timing is non-deterministic |
| "System prompt contains X" | Changes frequently, not a bug if different |
| Mock servers on network ports | Port conflicts, startup races |

### LLM Behavior is Not Unit Testable

The LLM's decisions (which tools to call, how to respond) are:
- Non-deterministic by design
- Model-specific
- Prompt-sensitive

**Don't mock the LLM for unit tests.** Instead:
- Test the machinery around it (parsers, routing, error handling)
- Use integration tests with real Ollama when available

## Test Organization

```
agent/
├── src/
│   ├── agent/
│   │   ├── parsers/
│   │   │   ├── mod.rs        # Parser registry
│   │   │   ├── json.rs       # JSON parser + tests
│   │   │   ├── xml.rs        # XML parser + tests
│   │   │   └── implicit.rs   # Implicit format + tests
│   │   └── mod.rs            # Agent + inline tests
│   ├── config.rs             # Config loading + tests
│   └── mcp/
│       ├── pool.rs           # Pool + tests
│       └── ...
└── tests/
    └── e2e/                  # Integration tests (require Ollama)
        ├── mod.rs
        ├── prerequisites.rs  # Check Ollama available
        └── ...
```

## Running Tests

```bash
# All unit tests (no external dependencies)
cargo test --workspace

# Skip E2E tests that require Ollama
cargo test --workspace --lib

# Run E2E tests (requires Ollama running)
cargo test --test e2e

# Run specific parser tests
cargo test --package agent parser
```

## CI Configuration

```yaml
# Unit tests always run
- run: cargo test --workspace --lib

# E2E tests are optional
- run: cargo test --test e2e
  continue-on-error: true  # Don't fail CI if Ollama unavailable
```

## Adding New Tests

When adding a test, ask yourself:

1. **Is this deterministic?** If it could fail randomly, don't write it.
2. **Does this test behavior or implementation?** Test behavior.
3. **Would a failure here indicate a real bug?** If not, skip it.
4. **Can this run without network/external services?** If not, mark as E2E.

## Examples

### Good Test
```rust
#[test]
fn parser_extracts_tool_name_from_json() {
    let input = r#"{"name": "get_weather", "arguments": {}}"#;
    let result = JsonParser::parse(input).unwrap();
    assert_eq!(result.name, "get_weather");
}
```

### Bad Test
```rust
#[test]
fn agent_calls_weather_tool() {
    // BAD: Depends on LLM deciding to call this specific tool
    let agent = Agent::new(real_ollama_client());
    let response = agent.process("What's the weather?").await;
    assert!(response.contains("weather")); // Flaky!
}
```

## Test Coverage Goals

Focus coverage on:
- All parser formats and edge cases
- All config loading paths
- All error handling branches

Don't chase coverage metrics for:
- LLM interaction code (tested via E2E)
- Simple struct definitions
- Generated code
