# Binks Global AI - Roadmap

## Vision

Build a **Global AI** that manages your entire digital life - not just code, but infrastructure, home automation, research, and more.

---

## Tool Strategy: Pre-built vs Custom

### The Hybrid Approach

We use a strategic mix of **pre-built tools** (for common operations) and **custom tools** (for domain-specific work).

```
┌─────────────────────────────────────────────────────────┐
│                    Master Agent                          │
├─────────────────────────────────────────────────────────┤
│  Pre-built Tools          │  Custom Tools               │
│  ─────────────────        │  ─────────────              │
│  ShellTools (general)     │  KubectlToolkit (K8s)       │
│  FileTools (read/write)   │  AgentSpawnerToolkit        │
│  [Future: GitTools]       │  [Future: HomeLabToolkit]   │
│  [Future: WebTools]       │  [Future: MonitoringToolkit]│
└─────────────────────────────────────────────────────────┘
```

### When to Use Pre-built Tools

| Scenario | Tool | Rationale |
|----------|------|-----------|
| Run shell commands | `ShellTools` | Covers 80% of coding tasks |
| Read/write files | `FileTools` | No need to reinvent |
| Git operations | `ShellTools` | git CLI is sufficient |
| Web searches | `DuckDuckGo` (future) | Pre-built, battle-tested |
| Database queries | `SQLToolkit` (future) | Standard SQL interface |

### When to Build Custom Tools

| Scenario | Custom Tool | Rationale |
|----------|-------------|-----------|
| Kubernetes management | `KubectlToolkit` | Domain-specific validation, error handling |
| Agent spawning | `AgentSpawnerToolkit` | Proprietary job templates |
| Home automation | `HomeLabToolkit` (future) | Specific device APIs |
| Monitoring/alerting | `MonitoringToolkit` (future) | Custom thresholds, integrations |

---

## Development Phases

### Phase 1: Crawl (Current)
**Goal:** Basic agent functionality with core tools

- [x] Master Agent with Ollama integration
- [x] Custom KubectlToolkit for K8s management
- [x] Custom AgentSpawnerToolkit for worker agents
- [x] Pre-built ShellTools for code editing
- [x] Pre-built FileTools for file operations
- [x] FastAPI server for remote access
- [ ] Basic error handling and retries

### Phase 2: Walk
**Goal:** Expanded capabilities and reliability

- [ ] Add pre-built `DuckDuckGo` or `GoogleSearch` toolkit
- [ ] Add pre-built `SQLToolkit` for database queries
- [ ] Implement agent memory/persistence
- [ ] Build `MonitoringToolkit` for cluster health alerts
- [ ] Add authentication to API
- [ ] Implement structured logging

### Phase 3: Run
**Goal:** Multi-agent orchestration and specialization

- [ ] Specialized worker agents:
  - Code Review Agent
  - Security Audit Agent
  - Research Agent
  - Home Automation Agent
- [ ] Agent-to-agent communication
- [ ] Task queue and scheduling
- [ ] MCP (Model Context Protocol) integration

### Phase 4: Fly
**Goal:** Full Global AI autonomy

- [ ] Proactive monitoring and auto-remediation
- [ ] Learning from past interactions
- [ ] Cross-domain orchestration (code + infra + home)
- [ ] Voice/natural interface options
- [ ] Mobile notifications and control

---

## Available Agno Pre-built Toolkits

Reference for future integration:

| Toolkit | Purpose | Phase |
|---------|---------|-------|
| `ShellTools` | Run terminal commands | 1 (done) |
| `FileTools` | Read/write files | 1 (done) |
| `DuckDuckGo` | Web search | 2 |
| `GoogleSearch` | Web search (API key required) | 2 |
| `SQLToolkit` | Database queries | 2 |
| `SlackToolkit` | Send Slack messages | 3 |
| `GithubToolkit` | Manage GitHub repos | 3 |
| `JiraToolkit` | Manage Jira tickets | 3 |

---

## MCP (Model Context Protocol) Strategy

MCP is an emerging standard for tool interoperability. Both Agno and Claude Code support it.

### Future MCP Integrations

```
┌─────────────────────────────────────────────────────────┐
│                    MCP Server Hub                        │
├─────────────────────────────────────────────────────────┤
│  Community MCP Servers    │  Custom MCP Servers         │
│  ─────────────────────    │  ──────────────────         │
│  Spotify Control          │  Home Lab Control           │
│  Smart Home (Home Asst.)  │  Pi Cluster Metrics         │
│  Calendar Management      │  Custom App APIs            │
└─────────────────────────────────────────────────────────┘
```

### Why MCP Matters

1. **Standardization** - One protocol, many tools
2. **Community** - Leverage others' work
3. **Portability** - Switch between agents without rewriting tools

---

## Architecture Decision Records

### ADR-001: Agno as Agent Framework
**Decision:** Use Agno as the agent framework
**Rationale:**
- Faster execution (no heavy abstractions)
- Simpler toolkit pattern
- Better suited for infrastructure orchestration
- Lightweight and resource-efficient

### ADR-002: Hybrid Tool Strategy
**Decision:** Use pre-built tools for common operations, custom tools for domain-specific work
**Rationale:**
- Pre-built tools are battle-tested and maintained
- Custom tools allow domain-specific optimization
- Reduces development time while maintaining flexibility

### ADR-003: Ollama for LLM
**Decision:** Use Ollama for local LLM hosting
**Rationale:**
- Privacy (no data leaves your network)
- Cost (no API fees)
- Control (choose your model)
- Performance (405B on M3 Ultra is viable)

---

## Contributing

When adding new capabilities:

1. **Check for pre-built toolkits first** - Don't reinvent the wheel
2. **Build custom only when needed** - Domain-specific, proprietary, or performance-critical
3. **Document in this roadmap** - Add to appropriate phase
4. **Consider MCP** - Could this be an MCP server others could use?

---

## CLI Orchestrator - Backend Providers

### Current Backends (December 2024)

| Backend | Status | Notes |
|---------|--------|-------|
| Claude Code | ✅ Ready | Primary backend |
| Gemini CLI | ✅ Ready | Installed |
| Groq | ✅ Ready | API runner, ultra-fast inference |
| OpenRouter | ✅ Ready | API runner, 20+ free models |
| Ollama Local | 🔧 Setup | Requires `ollama serve` |
| Ollama Home | 🔧 Setup | Remote at 192.168.1.4 |

### Active Service Projects

1. **Email Scoring LLM** ✅ COMPLETE
   - Score and categorize job application emails
   - Location: `services/email_scorer/`
   - See: `docs/projects/email-scoring-llm.md`

2. **JobScan AI Integration** ✅ COMPLETE
   - ATS scoring for LinkedIn job descriptions
   - Location: `services/jobscan/`
   - See: `docs/projects/jobscan-ai-integration.md`

### Phase: Free LLM Providers (After Services Built)

#### OpenRouter (Best for Variety) ✅ DONE
**20+ free models via single API**

```python
from runners import OpenRouterRunner

runner = OpenRouterRunner()  # Uses OPENROUTER_API_KEY env var
result = runner.run("Hello!")

# With specific model
runner = OpenRouterRunner(model="meta-llama/llama-3.1-8b-instruct:free")
```

Free models:
- `meta-llama/llama-3.1-8b-instruct:free`
- `mistralai/mistral-7b-instruct:free`
- `google/gemma-2-9b-it:free`
- `microsoft/phi-3-mini-128k-instruct:free`
- `qwen/qwen-2-7b-instruct:free`

**Tasks:**
- [x] Create `OpenRouterRunner` class
- [x] Add model selection for free tier
- [x] Handle rate limits gracefully
- [x] Integration tests added

#### Groq (Best for Speed) ✅ DONE
**Fast inference, generous free tier**

```python
from runners import GroqRunner

runner = GroqRunner()  # Uses GROQ_API_KEY env var
result = runner.run("Hello!")

# With specific model
runner = GroqRunner(model="llama-3.1-8b-instant")
```

Models: `llama-3.3-70b-versatile`, `llama-3.1-8b-instant`, `mixtral-8x7b-32768`, `gemma2-9b-it`

**Tasks:**
- [x] Create `GroqRunner` class
- [x] Optimized for multi-step agents (speed)
- [x] Integration tests added
- [ ] Benchmark against Claude/Gemini (future)

#### Google Gemini Free Tier (Best for Context)
**Already integrated - optimize for free tier**

```python
from agno.models.google import Gemini

model = Gemini(id="gemini-1.5-flash")
```

- 1M+ token context window
- Generous free rate limits

### Provider Comparison

| Provider | Speed | Free Models | Context | Best For |
|----------|-------|-------------|---------|----------|
| OpenRouter | Medium | 20+ | Varies | Experimentation |
| Groq | ⚡ Fast | 3-5 | 32K-128K | Multi-step agents |
| Gemini | Fast | 2-3 | 1M+ | Large documents |
| Claude | Medium | 0 | 200K | Quality/reasoning |
| Ollama | Varies | All local | Varies | Privacy/offline |

### Implementation Order

1. ~~**Now**: Use Claude + Gemini to build Email Scoring and JobScan services~~ ✅ DONE
2. ~~**Next**: Add Groq for speed improvements~~ ✅ DONE
3. ~~**Then**: Add OpenRouter for model variety~~ ✅ DONE
4. **Now**: Multi-provider fallback and cost tracking

### API Keys Required

| Provider | Environment Variable | Get Key From |
|----------|---------------------|--------------|
| Anthropic | `ANTHROPIC_API_KEY` | console.anthropic.com |
| Google | `GEMINI_API_KEY` | aistudio.google.com/apikey |
| Groq | `GROQ_API_KEY` | console.groq.com |
| OpenRouter | `OPENROUTER_API_KEY` | openrouter.ai/keys |

---

## References

- [Agno Documentation](https://docs.agno.com)
- [Agno Pre-built Toolkits](https://docs.agno.com/tools)
- [MCP Specification](https://modelcontextprotocol.io)
- [Goose vs Agno Comparison](./DUAL_IMPLEMENTATION_SUMMARY.md)
