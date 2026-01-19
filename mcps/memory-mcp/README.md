# Memory MCP

Persistent memory system for the Binks agent with dual-layer architecture.

## Overview

Memory MCP provides both session-scoped working memory and persistent knowledge storage across agent restarts. This enables the agent to:

- Track reasoning chains during complex tasks
- Remember context within a session
- Build up knowledge over time
- Learn from past interactions

## Architecture

```
┌─────────────────────────────────────────────┐
│              Memory MCP                      │
├─────────────────────────────────────────────┤
│  Session Layer (In-Memory)                   │
│  ┌─────────────────────────────────────────┐│
│  │ thoughts: Vec<Thought>      reasoning   ││
│  │ context: HashMap<K,V>       working mem ││
│  │ tool_results: Vec<Result>   recent ops  ││
│  └─────────────────────────────────────────┘│
├─────────────────────────────────────────────┤
│  Persistent Layer (SQLite)                   │
│  ┌─────────────────────────────────────────┐│
│  │ entities: KnowledgeGraph    facts       ││
│  │ summaries: Vec<Summary>     compressed  ││
│  │ preferences: UserPrefs      learned     ││
│  └─────────────────────────────────────────┘│
└─────────────────────────────────────────────┘
```

## Tools

### Session Memory (ephemeral)

| Tool | Parameters | Description |
|------|------------|-------------|
| `think` | `thought: string` | Record a reasoning step |
| `remember` | `key: string, value: any` | Store in working memory |
| `recall` | `key: string` | Retrieve from working memory |
| `get_context` | - | Get full session context |

### Persistent Memory (survives restarts)

| Tool | Parameters | Description |
|------|------------|-------------|
| `learn` | `entity: string, facts: string[]` | Add facts to knowledge graph |
| `query` | `pattern: string` | Search knowledge graph |
| `summarize_session` | - | Compress session to persistent storage |
| `forget` | `entity: string` | Remove entity from knowledge graph |

## Configuration

```json
{
  "mcpServers": {
    "memory": {
      "command": "./mcps/memory-mcp/target/release/memory-mcp",
      "env": {
        "MEMORY_DB_PATH": "${HOME}/.binks/memory.db",
        "SESSION_TIMEOUT_MINUTES": "60"
      }
    }
  }
}
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MEMORY_DB_PATH` | `~/.binks/memory.db` | SQLite database location |
| `SESSION_TIMEOUT_MINUTES` | `60` | Session expiry time |

## Data Model

### Session Layer

```rust
struct SessionMemory {
    session_id: Uuid,
    started_at: DateTime<Utc>,
    thoughts: Vec<Thought>,
    context: HashMap<String, serde_json::Value>,
    tool_results: Vec<ToolResult>,
}

struct Thought {
    id: u32,
    content: String,
    timestamp: DateTime<Utc>,
}
```

### Persistent Layer (SQLite Schema)

```sql
-- Knowledge graph entities
CREATE TABLE entities (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    entity_type TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Facts about entities
CREATE TABLE facts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id TEXT REFERENCES entities(id),
    fact TEXT NOT NULL,
    confidence REAL DEFAULT 1.0,
    source TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Relations between entities
CREATE TABLE relations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_entity TEXT REFERENCES entities(id),
    relation_type TEXT NOT NULL,
    to_entity TEXT REFERENCES entities(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Session summaries
CREATE TABLE summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT,
    summary TEXT NOT NULL,
    key_learnings TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## Usage Examples

### Recording thoughts during analysis

```json
// Agent analyzing a bug
{ "tool": "think", "arguments": { "thought": "The error occurs in auth.rs line 45" }}
{ "tool": "think", "arguments": { "thought": "This is a race condition in token refresh" }}
{ "tool": "remember", "arguments": { "key": "bug_location", "value": "auth.rs:45" }}
```

### Building knowledge

```json
// Learning about the codebase
{ "tool": "learn", "arguments": {
    "entity": "auth-service",
    "facts": [
      "Uses JWT tokens",
      "Token refresh every 15 minutes",
      "Located in src/auth/"
    ]
}}

// Later querying
{ "tool": "query", "arguments": { "pattern": "auth" }}
// Returns: auth-service entity with facts
```

### Session lifecycle

```json
// At end of task
{ "tool": "summarize_session", "arguments": {} }
// Compresses session to: "Fixed race condition in auth.rs token refresh"
// Stored in summaries table for future reference
```

## Building

```bash
cd mcps/memory-mcp
cargo build --release
```

## Dependencies

- `rmcp` - MCP SDK
- `rusqlite` - SQLite bindings
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `chrono` - Date/time handling
- `uuid` - Session IDs
