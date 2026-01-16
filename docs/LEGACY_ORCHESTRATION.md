# Legacy Orchestration Architecture

> **Note:** This document preserves the original Python-based orchestration design that was superseded by the Rust agent + MCP architecture. It's kept for historical reference and to document lessons learned.

## Original Vision

The Binks system was originally designed as a Python-based AI control plane running on an M3 Ultra, using the **Agno framework** for agent orchestration.

### Design Principles

1. **Separation of Concerns** - Clients, API, and Agent Core as independent layers
2. **Language Agnostic Clients** - Any language could be a client (Python, Rust, Go, JS)
3. **Single API Contract** - One REST/WebSocket API that all clients use
4. **Stateless API** - Agent state managed server-side, clients are thin

### Layered Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           CLIENTS                                    │
│   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  │
│   │   TUI   │  │   Web   │  │ Python  │  │  Rust   │  │ Mobile  │  │
│   │         │  │   UI    │  │   CLI   │  │   CLI   │  │   App   │  │
│   └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  │
└────────┼────────────┼────────────┼────────────┼────────────┼────────┘
         └────────────┴─────┬──────┴────────────┴────────────┘
                            │
                   HTTP/REST + WebSocket
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         API LAYER                                    │
│                      (FastAPI Server)                                │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │  REST Endpoints              │  WebSocket Endpoints          │  │
│   │  POST /api/v1/invoke         │  WS /api/v1/stream            │  │
│   │  POST /api/v1/invoke/stream  │  (real-time responses)        │  │
│   │  GET  /api/v1/health         │                               │  │
│   │  GET  /api/v1/agent/info     │                               │  │
│   └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────┬────────────────────────────────────┘
                                  │
                         Internal Python Calls
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        AGENT CORE                                    │
│                     (MasterAgent Class)                              │
│   - No CLI, no I/O, no print statements                             │
│   - Just agent logic and tool orchestration                         │
│   - Returns structured responses                                     │
│                                                                      │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │                      TOOLS                                    │  │
│   │  Pre-built (Agno):          Custom:                          │  │
│   │  ├── ShellTools             ├── KubectlToolkit               │  │
│   │  └── FileTools              └── AgentSpawnerToolkit          │  │
│   └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Agno Framework

The orchestrator used the **Agno** framework, which provided:

- **Pre-built tools**: ShellTools, FileTools for basic operations
- **Custom toolkits**: KubectlToolkit for Kubernetes, AgentSpawnerToolkit for spawning workers
- **Ollama integration**: Local LLM support via Ollama API

### Structure
```
orchestrator/
└── agno/
    ├── src/
    │   ├── agent.py          # Master Agent + CLI
    │   └── api/
    │       └── server.py     # FastAPI REST API
    ├── tools/
    │   ├── kubectl_tool.py   # Kubernetes commands
    │   └── agent_spawner.py  # Worker agent spawning
    └── requirements.txt
```

---

## API Contract (v1)

### REST Endpoints

#### Invoke Agent
```http
POST /api/v1/invoke
Content-Type: application/json

{
  "task": "Check cluster health and report any issues",
  "context": {
    "priority": "high",
    "namespace": "production"
  },
  "stream": false
}
```

Response:
```json
{
  "success": true,
  "request_id": "uuid-here",
  "result": "Cluster is healthy. All 5 nodes reporting Ready status...",
  "tool_calls": [
    {"tool": "get_cluster_status", "duration_ms": 1234}
  ],
  "duration_ms": 5678
}
```

#### Stream Response (SSE)
```http
POST /api/v1/invoke/stream
Content-Type: application/json

{
  "task": "Analyze all pods and suggest optimizations"
}
```

Response (Server-Sent Events):
```
data: {"type": "thinking", "content": "Analyzing pod configurations..."}
data: {"type": "tool_call", "tool": "run_kubectl", "args": {"command": "get pods -A"}}
data: {"type": "tool_result", "tool": "run_kubectl", "result": "..."}
data: {"type": "response", "content": "Based on my analysis..."}
data: {"type": "done", "request_id": "uuid-here"}
```

#### Health Check
```http
GET /api/v1/health
```

Response:
```json
{
  "status": "healthy",
  "agent": "ready",
  "ollama": "connected",
  "version": "1.0.0"
}
```

### WebSocket Endpoint

For real-time bidirectional communication:

```
WS /api/v1/stream
```

Client sends:
```json
{"type": "task", "task": "Monitor cluster for 5 minutes"}
```

Server sends (multiple messages):
```json
{"type": "thinking", "content": "Setting up monitoring..."}
{"type": "update", "content": "CPU usage: 45%"}
{"type": "done", "summary": "Monitoring complete. No anomalies detected."}
```

---

## Why We Evolved

The Python/Agno approach was superseded by a **Rust agent + MCP (Model Context Protocol)** architecture for several reasons:

### 1. Performance & Resource Efficiency
- Rust provides better memory management and lower overhead
- Important for running on edge devices and constrained environments

### 2. MCP Standardization
- MCP provides a standardized protocol for tool communication
- Tools can be developed independently and composed together
- Better ecosystem compatibility (Claude, other MCP clients)

### 3. Modular Tool Architecture
- MCPs are standalone servers that can be:
  - Developed in any language
  - Versioned independently
  - Shared across projects
  - Published to registries

### 4. Simplified Agent Core
- The Rust agent focuses purely on LLM interaction and tool orchestration
- Tools are external MCP servers, not embedded code
- Cleaner separation of concerns

---

## Current Architecture

The system now uses:

```
┌──────────────────┐     ┌──────────────────┐
│   Rust Agent     │────▶│   Ollama LLM     │
│  (tool-using)    │     │                  │
└────────┬─────────┘     └──────────────────┘
         │
         │ MCP Protocol (stdio)
         │
    ┌────┴────┬─────────┬─────────┐
    ▼         ▼         ▼         ▼
┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
│sysinfo│ │github │ │  k8s  │ │  ssh  │
│  mcp  │ │-gh mcp│ │  mcp  │ │  mcp  │
└───────┘ └───────┘ └───────┘ └───────┘
```

- **Agent**: Rust-based, uses `rmcp` crate for MCP client
- **MCPs**: Standalone servers exposing tools via MCP protocol
- **Configuration**: `.mcp.json` defines available MCP servers

---

## Lessons Learned

### What Worked Well
1. **Layered architecture** - The separation of clients/API/core was sound
2. **Structured API contract** - REST + WebSocket + SSE streaming was well-designed
3. **Tool abstraction** - The idea of composable tools was correct

### What We Changed
1. **Framework dependency** - Moved from Agno to direct MCP implementation
2. **Language** - Python → Rust for performance
3. **Tool hosting** - Embedded tools → External MCP servers
4. **Protocol** - Custom REST API → Standard MCP protocol

### Preserved Concepts
- Tool-using agent pattern
- Ollama as LLM backend
- Kubernetes integration
- Multi-client support (now via MCP)

---

## References

- [Model Context Protocol (MCP)](https://modelcontextprotocol.io/)
- [rmcp Rust crate](https://crates.io/crates/rmcp)
- [Ollama](https://ollama.ai/)
