# Filesystem MCP

Sandboxed file operations with security controls.

## Overview

Filesystem MCP provides secure file operations with:
- Allowlist-based directory access
- Path traversal prevention
- Size limits for operations
- Optional confirmation for destructive operations

## Tools

| Tool | Parameters | Description |
|------|------------|-------------|
| `read_file` | `path: string` | Read file contents |
| `write_file` | `path: string, content: string` | Write/create file |
| `list_dir` | `path: string, recursive?: bool` | List directory |
| `search_files` | `pattern: string, path?: string` | Search by pattern |
| `file_info` | `path: string` | Get file metadata |
| `move_file` | `src: string, dst: string` | Move/rename |
| `delete_file` | `path: string` | Delete file |

## Security Model

### Allowlist Configuration

```toml
# ~/.binks/filesystem.toml

[paths]
# Directories the agent can read from
read = [
    "~",
    "/tmp",
    "/var/log"
]

# Directories the agent can write to (subset of read)
write = [
    "~/projects",
    "~/dev",
    "/tmp"
]

# Directories never accessible
deny = [
    "~/.ssh",
    "~/.gnupg",
    "~/.aws"
]

[limits]
max_file_size = "10MB"
max_files_per_list = 1000
max_search_depth = 10

[safety]
confirm_delete = true
confirm_overwrite = false
backup_on_overwrite = false
```

### Path Validation

All paths are validated against:
1. **Canonicalization** - Resolve symlinks, `..`, `.`
2. **Allowlist check** - Must be under allowed directory
3. **Denylist check** - Cannot be in denied directories
4. **Traversal check** - No escaping allowed boundaries

## Configuration

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "./mcps/filesystem-mcp/target/release/filesystem-mcp",
      "env": {
        "FS_CONFIG_PATH": "${HOME}/.binks/filesystem.toml",
        "FS_DEFAULT_ALLOW": "${HOME}"
      }
    }
  }
}
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `FS_CONFIG_PATH` | `~/.binks/filesystem.toml` | Config file location |
| `FS_DEFAULT_ALLOW` | `~` | Default allowed directory |
| `FS_MAX_FILE_SIZE` | `10485760` (10MB) | Max file size in bytes |

## Usage Examples

### Reading files

```json
{ "tool": "read_file", "arguments": { "path": "~/projects/app/src/main.rs" }}
// Returns: file contents as string
```

### Writing files

```json
{ "tool": "write_file", "arguments": {
    "path": "~/projects/app/src/config.rs",
    "content": "pub const VERSION: &str = \"1.0.0\";"
}}
// Returns: { "success": true, "bytes_written": 35 }
```

### Listing directories

```json
{ "tool": "list_dir", "arguments": { "path": "~/projects/app/src", "recursive": true }}
// Returns: [
//   { "name": "main.rs", "type": "file", "size": 1234 },
//   { "name": "lib.rs", "type": "file", "size": 567 },
//   { "name": "utils/", "type": "directory" },
//   { "name": "utils/helpers.rs", "type": "file", "size": 890 }
// ]
```

### Searching files

```json
{ "tool": "search_files", "arguments": { "pattern": "*.rs", "path": "~/projects" }}
// Returns: list of matching file paths
```

## Error Handling

| Error | Description |
|-------|-------------|
| `ACCESS_DENIED` | Path not in allowlist or in denylist |
| `PATH_TRAVERSAL` | Attempted to escape allowed directory |
| `FILE_TOO_LARGE` | File exceeds size limit |
| `NOT_FOUND` | File or directory doesn't exist |
| `PERMISSION_ERROR` | OS-level permission denied |

## Building

```bash
cd mcps/filesystem-mcp
cargo build --release
```

## Dependencies

- `rmcp` - MCP SDK
- `tokio` - Async runtime (fs operations)
- `serde` / `toml` - Configuration
- `glob` - Pattern matching
- `dirs` - Home directory resolution
