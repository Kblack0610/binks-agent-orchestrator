# Web Search MCP Benchmarks

Comparison between the self-hosted SearXNG MCP and Claude's built-in WebSearch tool.

**Date:** 2026-01-23
**SearXNG Version:** Latest (Docker)
**Backend:** SearXNG with brave, google, startpage engines

---

## Summary

| Metric | SearXNG MCP | Claude WebSearch |
|--------|-------------|------------------|
| Average Results | 5 per query | 10 per query |
| Response Format | JSON with metadata | JSON + synthesized summary |
| News Search | Supported | Not directly available |
| Image Search | Supported | Not directly available |
| Privacy | Self-hosted, no tracking | Anthropic infrastructure |
| Cost | Free (self-hosted) | Included with Claude |
| Latency | ~200-500ms | ~500-1000ms |

---

## Detailed Comparison Tests

### Test 1: Technical Documentation Query
**Query:** `kubernetes pod restart policy`

| Aspect | SearXNG MCP | Claude WebSearch |
|--------|-------------|------------------|
| Results Count | 5 | 10 |
| Top Result | kubernetes.io (official docs) | kubernetes.io (official docs) |
| Quality | High - official docs first | High - includes tutorials |
| Descriptions | Excerpts from pages | Full synthesized explanation |

**Observations:**
- Both return the official Kubernetes documentation as the top result
- Claude provides a comprehensive synthesized answer with code examples
- SearXNG returns raw results faster, requiring less token usage

---

### Test 2: Current Events Query
**Query:** `latest AI news January 2025`

| Aspect | SearXNG MCP | Claude WebSearch |
|--------|-------------|------------------|
| Results Count | 5 | 10 |
| Freshness | Mixed (some dated) | Current and relevant |
| Sources | Google Blog, Reuters, Microsoft | TIME, PBS, MIT Tech Review |
| Quality | Good variety | Better curation |

**Observations:**
- Claude's WebSearch provides better news curation with synthesized summaries
- SearXNG returns broader source variety but less focused
- For breaking news, dedicated news search (`search_news`) works better

---

### Test 3: Programming Help Query
**Query:** `how to fix rust borrow checker error`

| Aspect | SearXNG MCP | Claude WebSearch |
|--------|-------------|------------------|
| Results Count | 5 | 10 |
| Top Sources | Stack Overflow, Medium, Rust Forum | Medium, Official Docs, GitHub |
| Practical Help | Good - links to solutions | Excellent - synthesized guide |
| Code Examples | Via links | Inline in response |

**Observations:**
- Both return relevant programming resources
- Claude's synthesis provides actionable debugging steps
- SearXNG is faster for quick link discovery

---

### Test 4: Niche Technical Comparison
**Query:** `obscure programming language zig vs odin comparison`

| Aspect | SearXNG MCP | Claude WebSearch |
|--------|-------------|------------------|
| Results Count | 5 | 10 |
| Forum Coverage | Reddit, Odin Forum | Odin Forum, Hacker News |
| Blog Posts | Ayende's notes | Same + HackerNoon |
| Benchmarks | Not included | Benchmark link included |

**Observations:**
- Both handle niche queries well
- Similar source discovery for specialized topics
- Claude includes benchmark comparison links

---

### Test 5: Site-Specific Search
**Query:** `site:github.com MCP server typescript`

| Aspect | SearXNG MCP | Claude WebSearch |
|--------|-------------|------------------|
| Results Count | 5 | 10 |
| Official Repos | typescript-sdk | typescript-sdk |
| Templates | 2 templates | 5+ templates |
| Frameworks | fastmcp | fastmcp, easy-mcp |

**Observations:**
- Both respect site: operator correctly
- Claude returns more comprehensive GitHub coverage
- SearXNG sufficient for finding official repositories

---

### Test 6: News Search (SearXNG Only)
**Query:** `technology startup funding`

| Aspect | SearXNG MCP |
|--------|-------------|
| Results Count | 5 |
| Freshness | 1-3 days old |
| Sources | MSN, GeekWire, SiliconANGLE, Defense News |
| Published Dates | Included |

**Observations:**
- Dedicated news endpoint provides fresh, dated results
- Claude WebSearch doesn't have a dedicated news mode
- Useful for monitoring current events

---

### Test 7: Image Search (SearXNG Only)
**Query:** `rust programming language logo`

| Aspect | SearXNG MCP |
|--------|-------------|
| Results Count | 3 |
| Sources | VectorSeek, Etsy, VHV |
| Metadata | image_url, page_url, source |
| Formats | PNG, JPG |

**Observations:**
- Functional image search with direct URLs
- Claude WebSearch doesn't support image search
- Useful for finding visual assets

---

## Performance Characteristics

### Response Time (Informal)
| Query Type | SearXNG MCP | Claude WebSearch |
|------------|-------------|------------------|
| Simple query | ~200ms | ~500ms |
| Complex query | ~400ms | ~800ms |
| News search | ~300ms | N/A |
| Image search | ~350ms | N/A |

### Token Usage
| Metric | SearXNG MCP | Claude WebSearch |
|--------|-------------|------------------|
| Response size | ~500-1000 tokens | ~1500-3000 tokens |
| Includes synthesis | No | Yes |
| Structured data | Yes (JSON) | Partial |

---

## Strengths & Weaknesses

### SearXNG MCP Strengths
1. **Privacy** - Self-hosted, no data sent to third parties
2. **Speed** - Lower latency for raw results
3. **Flexibility** - News and image search modes
4. **Token Efficiency** - Smaller responses, structured JSON
5. **No Rate Limits** - Self-hosted means unlimited queries
6. **Multi-Engine** - Aggregates results from multiple search engines
7. **Customizable** - Full control over engines and settings

### SearXNG MCP Weaknesses
1. **No Synthesis** - Returns raw links, no AI summarization
2. **Fewer Results** - Default 5-10 vs Claude's richer output
3. **Infrastructure** - Requires running Docker container
4. **Configuration** - Needs setup (JSON format, secret key, etc.)
5. **Engine Reliability** - Some engines may be blocked or rate-limited

### Claude WebSearch Strengths
1. **AI Synthesis** - Provides summarized, actionable answers
2. **No Setup** - Works out of the box
3. **Better Curation** - More relevant result ordering
4. **Inline Citations** - Sources linked in summaries
5. **Maintained** - Anthropic handles infrastructure

### Claude WebSearch Weaknesses
1. **Privacy** - Queries go through Anthropic
2. **No Specialized Modes** - No dedicated news/image search
3. **Higher Latency** - Synthesis adds processing time
4. **Token Heavy** - Larger responses use more context
5. **Opaque** - No control over search engine selection

---

## Recommendations for Improvement

### High Priority

#### 1. Add Result Synthesis Option
```rust
// Add optional AI summarization using local LLM
pub struct SearchParams {
    pub query: String,
    pub limit: usize,
    pub summarize: bool,  // NEW: optionally summarize results
}
```
- Integrate with Ollama for local summarization
- Make synthesis optional to preserve speed when not needed

#### 2. Improve Result Ranking
- Implement result deduplication across engines
- Add relevance scoring based on query terms
- Prioritize authoritative sources (official docs, .gov, .edu)

#### 3. Add Caching with TTL
```rust
// Cache results to reduce SearXNG load
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub max_entries: usize,
}
```
- Cache identical queries for configurable duration
- Reduce latency for repeated searches

### Medium Priority

#### 4. Enhanced Metadata
- Extract publication dates more reliably
- Add domain authority indicators
- Include result position/rank from source engine

#### 5. Query Enhancement
```rust
// Auto-enhance queries for better results
fn enhance_query(query: &str) -> String {
    // Add date filters for news-like queries
    // Expand acronyms
    // Handle typos
}
```

#### 6. Fallback Engines
- Implement engine failover when primary engines fail
- Add health checking for configured engines
- Automatic retry with alternative engines

### Low Priority

#### 7. Result Filtering
- Add safe search levels
- Domain blocklist/allowlist
- Content type filtering

#### 8. Analytics
- Track query patterns
- Monitor engine response times
- Log failed queries for debugging

#### 9. Streaming Results
- Return results as they arrive from engines
- Don't wait for all engines to respond

---

## Configuration Recommendations

### Optimal SearXNG Settings
```yaml
search:
  formats:
    - html
    - json
  safe_search: 0
  default_lang: "en"

server:
  limiter: false  # Disable for local use

engines:
  - name: google
    weight: 1.5
  - name: brave
    weight: 1.2
  - name: duckduckgo
    weight: 1.0
```

### Recommended Engine Mix
| Use Case | Engines |
|----------|---------|
| General | google, brave, duckduckgo |
| Technical | google, stackoverflow, github |
| News | google news, bing news, yahoo news |
| Images | bing images, google images |

---

## Conclusion

The SearXNG MCP is a solid self-hosted alternative to Claude's built-in WebSearch. It excels in:
- **Privacy-conscious use cases**
- **High-volume searching** (no rate limits)
- **Specialized searches** (news, images)
- **Token-efficient workflows**

Claude's WebSearch is better for:
- **End-user facing responses** (synthesized answers)
- **Zero-configuration deployments**
- **Complex research queries** (better curation)

**Recommendation:** Use SearXNG MCP for automated/programmatic searches and privacy-sensitive workflows. Use Claude WebSearch when you need synthesized, human-readable answers.

---

## Test Environment

- **OS:** Linux 6.18.6-2-cachyos
- **SearXNG:** Docker container (searxng/searxng:latest)
- **MCP:** web-search-mcp v0.1.0
- **Engines Enabled:** google, brave, startpage, bing, duckduckgo
