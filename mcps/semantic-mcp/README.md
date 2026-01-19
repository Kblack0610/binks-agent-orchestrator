# Semantic MCP

Code understanding and navigation using tree-sitter.

## Overview

Semantic MCP provides lightweight code analysis using tree-sitter for fast, multi-language parsing. For advanced LSP-powered analysis (refactoring, cross-file references), use Serena instead.

## Tools

| Tool | Parameters | Description |
|------|------------|-------------|
| `get_symbols` | `file: string` | List symbols in file |
| `find_definition` | `symbol: string, scope?: string` | Find where defined |
| `find_references` | `symbol: string, scope?: string` | Find usages |
| `get_outline` | `file: string` | File structure/outline |
| `parse_function` | `file: string, name: string` | Parse specific function |
| `get_imports` | `file: string` | List imports/dependencies |

## Supported Languages

| Language | Parser | Symbols |
|----------|--------|---------|
| Rust | tree-sitter-rust | functions, structs, enums, traits, impls |
| TypeScript | tree-sitter-typescript | functions, classes, interfaces, types |
| Python | tree-sitter-python | functions, classes, methods |
| Go | tree-sitter-go | functions, structs, interfaces |
| JavaScript | tree-sitter-javascript | functions, classes |

## Configuration

```json
{
  "mcpServers": {
    "semantic": {
      "command": "./mcps/semantic-mcp/target/release/semantic-mcp",
      "env": {
        "SEMANTIC_CACHE_SIZE": "100"
      }
    }
  }
}
```

## Usage Examples

### Get file symbols

```json
{ "tool": "get_symbols", "arguments": { "file": "src/auth.rs" }}
// Returns: [
//   { "name": "AuthService", "kind": "struct", "line": 10 },
//   { "name": "AuthService::new", "kind": "function", "line": 15 },
//   { "name": "AuthService::verify", "kind": "function", "line": 25 },
//   { "name": "Token", "kind": "struct", "line": 50 }
// ]
```

### Get file outline

```json
{ "tool": "get_outline", "arguments": { "file": "src/auth.rs" }}
// Returns:
// AuthService (struct)
//   ├── new() -> Self
//   ├── verify(&self, token: &str) -> bool
//   └── refresh(&self) -> Token
// Token (struct)
//   └── is_expired(&self) -> bool
```

### Find definition

```json
{ "tool": "find_definition", "arguments": { "symbol": "AuthService" }}
// Returns: { "file": "src/auth.rs", "line": 10, "column": 1 }
```

### Find references

```json
{ "tool": "find_references", "arguments": {
    "symbol": "verify",
    "scope": "src/"
}}
// Returns: [
//   { "file": "src/main.rs", "line": 45, "context": "auth.verify(token)" },
//   { "file": "src/api.rs", "line": 23, "context": "self.auth.verify(&t)" }
// ]
```

### Parse function

```json
{ "tool": "parse_function", "arguments": {
    "file": "src/auth.rs",
    "name": "verify"
}}
// Returns: {
//   "name": "verify",
//   "params": [{ "name": "self", "type": "&self" }, { "name": "token", "type": "&str" }],
//   "return_type": "bool",
//   "body_start": 26,
//   "body_end": 35,
//   "doc_comment": "Verifies a JWT token..."
// }
```

## vs Serena

| Feature | Semantic MCP | Serena |
|---------|--------------|--------|
| Parsing | tree-sitter | LSP |
| Speed | Fast | Slower (LSP startup) |
| Languages | Limited | Many (via LSP) |
| Refactoring | No | Yes |
| Cross-file refs | Basic | Full |
| Type inference | No | Yes |
| Setup | Standalone | Requires LSP |

**Use Semantic MCP for:** Quick symbol lookup, file analysis, basic navigation
**Use Serena for:** Refactoring, deep type analysis, cross-file understanding

## Architecture

```
┌─────────────────────────────────────────┐
│           Semantic MCP                   │
├─────────────────────────────────────────┤
│  Language Detection                      │
│  (by extension)                          │
├─────────────────────────────────────────┤
│  Tree-Sitter Parsers                     │
│  ┌─────┐ ┌────┐ ┌──────┐ ┌────┐        │
│  │Rust │ │TS  │ │Python│ │ Go │        │
│  └─────┘ └────┘ └──────┘ └────┘        │
├─────────────────────────────────────────┤
│  Symbol Extraction                       │
│  (language-specific queries)             │
├─────────────────────────────────────────┤
│  Cache Layer                             │
│  (parsed ASTs, symbol tables)            │
└─────────────────────────────────────────┘
```

## Integrating with Serena

For complex tasks, you can use both:

```
1. Quick lookup → Semantic MCP (fast)
2. Deep analysis → Serena (comprehensive)
```

Configure both in `.mcp.json` and choose based on task complexity.

## Building

```bash
cd mcps/semantic-mcp
cargo build --release
```

## Dependencies

- `rmcp` - MCP SDK
- `tree-sitter` - Core parsing
- `tree-sitter-rust` - Rust parser
- `tree-sitter-typescript` - TS/JS parser
- `tree-sitter-python` - Python parser
- `tree-sitter-go` - Go parser
- `tokio` - Async runtime
- `serde` - Serialization
