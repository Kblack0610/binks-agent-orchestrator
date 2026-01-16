# GitHub CLI MCP Server

A fast Rust-based MCP (Model Context Protocol) server that wraps the GitHub CLI (`gh`) to enable GitHub operations for GitHub Enterprise with OAuth authentication.

## Features

- **Issues**: List, view, create, edit, and close issues
- **Pull Requests**: List, view, create, and merge PRs
- **Workflows**: List workflows, trigger runs, view run status, cancel runs
- **Repositories**: List and view repository information

## Requirements

- GitHub CLI (`gh`) installed and in PATH
- `gh` authenticated (run `gh auth login`)
- Rust toolchain (for building from source)

## Building

```bash
cd mcps/github-gh
cargo build --release
```

The binary will be at `target/release/github-gh-mcp` (~3MB).

## Configuration

Add to your `.mcp.json`:

```json
{
  "mcpServers": {
    "github-gh": {
      "command": "./mcps/github-gh/target/release/github-gh-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

## Available Tools

### Issues
| Tool | Description |
|------|-------------|
| `gh_issue_list` | List issues with filters (state, assignee, label) |
| `gh_issue_view` | View issue details |
| `gh_issue_create` | Create a new issue |
| `gh_issue_edit` | Edit an existing issue |
| `gh_issue_close` | Close an issue |

### Pull Requests
| Tool | Description |
|------|-------------|
| `gh_pr_list` | List PRs with filters |
| `gh_pr_view` | View PR details |
| `gh_pr_create` | Create a new PR |
| `gh_pr_merge` | Merge a PR |

### Workflows/Actions
| Tool | Description |
|------|-------------|
| `gh_workflow_list` | List workflows |
| `gh_workflow_run` | Trigger a workflow |
| `gh_run_list` | List workflow runs |
| `gh_run_view` | View run details |
| `gh_run_cancel` | Cancel a run |

### Repositories
| Tool | Description |
|------|-------------|
| `gh_repo_list` | List repositories |
| `gh_repo_view` | View repo details |

## Example Usage

Once configured, you can use these tools through Claude Code:

```
"List open issues in my-org/my-repo"
"Create an issue titled 'Bug fix needed' in owner/repo"
"View PR #123 in owner/repo"
"Merge PR #456 in owner/repo using squash"
```

## Development

```bash
# Build debug version
cargo build

# Run with debug logging
RUST_LOG=debug cargo run
```

## License

MIT
