# Git MCP

Local git operations using libgit2.

## Overview

Git MCP provides local repository operations that complement the GitHub API-based `github-gh` MCP. Use this for:

- Repository status and diffs
- Commit history analysis
- Blame and change tracking
- Branch management
- Stash operations

## Tools

| Tool | Parameters | Description |
|------|------------|-------------|
| `git_status` | `repo: string` | Get working tree status |
| `git_diff` | `repo: string, ref?: string` | Show diff against ref |
| `git_log` | `repo: string, limit?: int` | Commit history |
| `git_blame` | `repo: string, file: string` | Blame for file |
| `git_show` | `repo: string, ref: string` | Show commit details |
| `git_branch_list` | `repo: string` | List branches |
| `git_stash` | `repo: string, action: string` | Stash operations |

## Configuration

```json
{
  "mcpServers": {
    "git": {
      "command": "./mcps/git-mcp/target/release/git-mcp",
      "env": {
        "GIT_DEFAULT_REPO": "${PWD}"
      }
    }
  }
}
```

## Usage Examples

### Get repository status

```json
{ "tool": "git_status", "arguments": { "repo": "~/projects/myapp" }}
// Returns:
// {
//   "branch": "feature/auth",
//   "ahead": 2,
//   "behind": 0,
//   "staged": ["src/auth.rs"],
//   "modified": ["src/main.rs"],
//   "untracked": ["tests/auth_test.rs"]
// }
```

### View diff

```json
{ "tool": "git_diff", "arguments": { "repo": "~/projects/myapp" }}
// Returns: unified diff of working tree

{ "tool": "git_diff", "arguments": { "repo": "~/projects/myapp", "ref": "main" }}
// Returns: diff between HEAD and main
```

### Commit history

```json
{ "tool": "git_log", "arguments": { "repo": "~/projects/myapp", "limit": 5 }}
// Returns: [
//   { "sha": "abc123", "message": "Add auth", "author": "user", "date": "..." },
//   ...
// ]
```

### Blame

```json
{ "tool": "git_blame", "arguments": { "repo": "~/projects/myapp", "file": "src/auth.rs" }}
// Returns: line-by-line blame information
```

### Branch list

```json
{ "tool": "git_branch_list", "arguments": { "repo": "~/projects/myapp" }}
// Returns: [
//   { "name": "main", "current": false, "remote": "origin/main" },
//   { "name": "feature/auth", "current": true, "remote": null }
// ]
```

### Stash operations

```json
{ "tool": "git_stash", "arguments": { "repo": "~/projects/myapp", "action": "list" }}
{ "tool": "git_stash", "arguments": { "repo": "~/projects/myapp", "action": "push" }}
{ "tool": "git_stash", "arguments": { "repo": "~/projects/myapp", "action": "pop" }}
```

## vs github-gh MCP

| Operation | git-mcp | github-gh |
|-----------|---------|-----------|
| Local status | Yes | No |
| Local diff | Yes | PR diff only |
| Commit history | Local | API-based |
| Blame | Yes | No |
| Create PR | No | Yes |
| Issues | No | Yes |
| Workflows | No | Yes |

**Use together:** git-mcp for local analysis, github-gh for remote operations.

## Building

```bash
cd mcps/git-mcp
cargo build --release
```

## Dependencies

- `rmcp` - MCP SDK
- `git2` - libgit2 bindings (core functionality)
- `tokio` - Async runtime
- `serde` - Serialization
