# Web Fetch MCP

HTTP fetching and HTML parsing.

## Overview

Web Fetch MCP provides simple HTTP operations for:

- Fetching web pages and APIs
- Parsing HTML with CSS selectors
- Converting HTML to markdown
- JSON API interactions

For complex browser automation (JS rendering, interactions), use Playwright MCP instead.

## Tools

| Tool | Parameters | Description |
|------|------------|-------------|
| `fetch` | `url: string, headers?: object` | Fetch raw content |
| `fetch_json` | `url: string, headers?: object` | Fetch and parse JSON |
| `parse_html` | `url: string, selector: string` | Extract via CSS selector |
| `fetch_markdown` | `url: string` | Convert HTML to markdown |

## Configuration

```json
{
  "mcpServers": {
    "web-fetch": {
      "command": "./mcps/web-fetch-mcp/target/release/web-fetch-mcp",
      "env": {
        "HTTP_TIMEOUT_SECONDS": "30",
        "USER_AGENT": "BinksAgent/1.0"
      }
    }
  }
}
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `HTTP_TIMEOUT_SECONDS` | `30` | Request timeout |
| `USER_AGENT` | `BinksAgent/1.0` | User-Agent header |
| `MAX_RESPONSE_SIZE` | `10485760` (10MB) | Max response size |

## Usage Examples

### Basic fetch

```json
{ "tool": "fetch", "arguments": { "url": "https://example.com" }}
// Returns: raw HTML string
```

### Fetch JSON API

```json
{ "tool": "fetch_json", "arguments": {
    "url": "https://api.github.com/repos/owner/repo",
    "headers": { "Authorization": "Bearer token" }
}}
// Returns: parsed JSON object
```

### Parse HTML with selector

```json
{ "tool": "parse_html", "arguments": {
    "url": "https://docs.rs/tokio",
    "selector": "h1, h2, h3"
}}
// Returns: array of matched elements' text
```

### Convert to markdown

```json
{ "tool": "fetch_markdown", "arguments": { "url": "https://rust-lang.org" }}
// Returns: markdown version of the page
```

## vs Playwright MCP

| Feature | web-fetch-mcp | Playwright |
|---------|---------------|------------|
| Static HTML | Yes | Yes |
| JS rendering | No | Yes |
| Interactions | No | Yes |
| Screenshots | No | Yes |
| Speed | Fast | Slower |
| Dependencies | Minimal | Chromium |

**Use web-fetch for:** Documentation, APIs, static pages
**Use Playwright for:** SPAs, JS-heavy sites, testing

## Building

```bash
cd mcps/web-fetch-mcp
cargo build --release
```

## Dependencies

- `rmcp` - MCP SDK
- `reqwest` - HTTP client
- `scraper` - HTML parsing
- `html2md` - HTML to markdown
- `tokio` - Async runtime
- `serde` - Serialization
