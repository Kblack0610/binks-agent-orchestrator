# Test Directory

This directory contains integration tests and test utilities for the Binks system.

## Structure

```
test/
├── README.md           # This file
├── integration/        # Integration tests
│   └── ...
└── fixtures/           # Test data and fixtures
    └── ...
```

## Running Tests

### Agent Tests
```bash
cd agent
cargo test
```

### MCP Server Tests
```bash
cd mcps/sysinfo-mcp
cargo test

cd mcps/github-gh
cargo test
```

## Integration Testing

Integration tests verify the full system works together:

1. Agent can connect to MCP servers
2. Tools return expected results
3. Configuration is loaded correctly

## Adding Tests

- Unit tests go in the respective `src/` directories (Rust convention)
- Integration tests that span multiple components go here
- Test fixtures and mock data go in `fixtures/`
