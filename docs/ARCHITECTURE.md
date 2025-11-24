# Binks Architecture

## Design Principles

1. **Separation of Concerns** - Clients, API, and Agent Core are independent layers
2. **Language Agnostic Clients** - Any language can be a client (Python, Rust, Go, JS)
3. **Single API Contract** - One REST/WebSocket API that all clients use
4. **Stateless API** - Agent state managed server-side, clients are thin

---

## Target Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           CLIENTS                                    │
│                                                                      │
│   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  │
│   │   TUI   │  │   Web   │  │ Python  │  │  Rust   │  │ Mobile  │  │
│   │(current)│  │   UI    │  │   CLI   │  │   CLI   │  │   App   │  │
│   └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  │
│        │            │            │            │            │        │
└────────┼────────────┼────────────┼────────────┼────────────┼────────┘
         │            │            │            │            │
         └────────────┴─────┬──────┴────────────┴────────────┘
                            │
                   HTTP/REST + WebSocket
                   (Language Agnostic)
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         API LAYER                                    │
│                    (orchestrator/agno/src/api/)                      │
│                                                                      │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │                     FastAPI Server                            │  │
│   ├──────────────────────────────────────────────────────────────┤  │
│   │  REST Endpoints              │  WebSocket Endpoints          │  │
│   │  ─────────────────           │  ────────────────────         │  │
│   │  POST /api/v1/invoke         │  WS /api/v1/stream            │  │
│   │  POST /api/v1/invoke/stream  │  (real-time responses)        │  │
│   │  GET  /api/v1/health         │                               │  │
│   │  GET  /api/v1/agent/info     │                               │  │
│   │  POST /api/v1/cluster/status │                               │  │
│   └──────────────────────────────────────────────────────────────┘  │
│                                                                      │
└─────────────────────────────────┬────────────────────────────────────┘
                                  │
                         Internal Python Calls
                         (NOT exposed externally)
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        AGENT CORE                                    │
│                   (orchestrator/agno/src/core/)                      │
│                                                                      │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │                    MasterAgent                                │  │
│   │                 (Pure Python Class)                           │  │
│   │                                                               │  │
│   │  - No CLI, no I/O, no print statements                       │  │
│   │  - Just agent logic and tool orchestration                   │  │
│   │  - Returns structured responses                              │  │
│   └──────────────────────────────────────────────────────────────┘  │
│                                                                      │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │                      TOOLS                                    │  │
│   │                                                               │  │
│   │  Pre-built:              Custom:                             │  │
│   │  ├── ShellTools          ├── KubectlToolkit                  │  │
│   │  └── FileTools           └── AgentSpawnerToolkit             │  │
│   └──────────────────────────────────────────────────────────────┘  │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Layer Responsibilities

### 1. Client Layer
**Location:** `client/` (and external repos for other languages)

**Responsibility:**
- User interface (TUI, Web, CLI)
- User input/output
- Display formatting
- NO business logic

**Can be written in:** Any language (Python, Rust, Go, TypeScript, Swift)

**Communicates via:** HTTP REST + WebSocket only

### 2. API Layer
**Location:** `orchestrator/agno/src/api/`

**Responsibility:**
- HTTP/WebSocket endpoints
- Request validation (Pydantic models)
- Authentication/Authorization (future)
- Rate limiting (future)
- Streaming responses
- Error handling and HTTP status codes

**Key Files:**
```
api/
├── __init__.py
├── server.py          # FastAPI app initialization
├── routes/
│   ├── __init__.py
│   ├── agent.py       # /invoke, /stream endpoints
│   ├── cluster.py     # /cluster/* endpoints
│   └── health.py      # /health endpoint
├── websocket.py       # WebSocket handlers
├── models.py          # Pydantic request/response models
└── middleware.py      # Auth, logging, etc.
```

### 3. Agent Core Layer
**Location:** `orchestrator/agno/src/core/`

**Responsibility:**
- Agent logic and reasoning
- Tool orchestration
- LLM interaction
- NO I/O (no print, no input, no CLI)

**Key Files:**
```
core/
├── __init__.py
├── agent.py           # MasterAgent class (pure logic)
├── config.py          # Agent configuration
└── tools/
    ├── __init__.py
    ├── kubectl.py     # KubectlToolkit
    └── spawner.py     # AgentSpawnerToolkit
```

---

## API Contract (v1)

All clients use this contract. Language doesn't matter.

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

For real-time bidirectional communication (useful for TUI):

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
{"type": "update", "content": "CPU usage: 52%"}
{"type": "done", "summary": "Monitoring complete. No anomalies detected."}
```

---

## Directory Structure (Target)

```
binks/
├── orchestrator/
│   └── agno/
│       ├── src/
│       │   ├── api/                 # API Layer
│       │   │   ├── __init__.py
│       │   │   ├── server.py        # FastAPI app
│       │   │   ├── routes/
│       │   │   │   ├── agent.py
│       │   │   │   ├── cluster.py
│       │   │   │   └── health.py
│       │   │   ├── websocket.py
│       │   │   └── models.py
│       │   │
│       │   └── core/                # Agent Core Layer
│       │       ├── __init__.py
│       │       ├── agent.py         # Pure agent logic
│       │       ├── config.py
│       │       └── tools/
│       │           ├── kubectl.py
│       │           └── spawner.py
│       │
│       ├── requirements.txt
│       └── README.md
│
├── client/                          # Client Layer (Python reference)
│   ├── src/
│   │   ├── simple_client.py         # CLI client
│   │   └── lib/
│   │       └── binks_client.py      # Reusable client library
│   └── config/
│
├── clients/                         # Future: Other language clients
│   ├── rust-cli/
│   ├── web-ui/
│   └── mobile/
│
└── docs/
    ├── ARCHITECTURE.md              # This file
    ├── API_REFERENCE.md             # Detailed API docs
    └── CLIENT_GUIDE.md              # How to build a client
```

---

## Migration Path

### Current State
```
agent.py = Agent logic + CLI (mixed)
server.py = API layer (good)
simple_client.py = Python client (good)
```

### Step 1: Extract Core
```bash
# Move pure agent logic to core/
orchestrator/agno/src/core/agent.py  # No CLI, no I/O

# Keep API layer
orchestrator/agno/src/api/server.py  # Imports from core/
```

### Step 2: Remove CLI from Agent
```python
# BEFORE (agent.py)
def main():
    while True:
        user_input = input("You: ")  # BAD: I/O in agent
        ...

# AFTER (core/agent.py)
class MasterAgent:
    def invoke(self, task: str) -> AgentResponse:
        # Pure logic, returns structured response
        return AgentResponse(...)
```

### Step 3: Add Streaming Support
```python
# api/routes/agent.py
@router.post("/invoke/stream")
async def invoke_stream(request: TaskRequest):
    async def generate():
        async for chunk in agent.invoke_stream(request.task):
            yield f"data: {chunk.json()}\n\n"
    return StreamingResponse(generate(), media_type="text/event-stream")
```

---

## Client Implementation Guide

### Python Client (Reference)
Already exists at `client/src/simple_client.py`

### Rust Client (Example Structure)
```rust
// binks-cli/src/main.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct TaskRequest {
    task: String,
}

#[derive(Deserialize)]
struct TaskResponse {
    success: bool,
    result: String,
}

async fn invoke(client: &Client, base_url: &str, task: &str) -> Result<TaskResponse> {
    let resp = client
        .post(format!("{}/api/v1/invoke", base_url))
        .json(&TaskRequest { task: task.to_string() })
        .send()
        .await?
        .json()
        .await?;
    Ok(resp)
}
```

### Web Client (Example)
```typescript
// web-ui/src/lib/binks.ts
export async function invoke(task: string): Promise<TaskResponse> {
  const response = await fetch('/api/v1/invoke', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ task }),
  });
  return response.json();
}

// With streaming
export async function* invokeStream(task: string) {
  const response = await fetch('/api/v1/invoke/stream', {
    method: 'POST',
    body: JSON.stringify({ task }),
  });
  const reader = response.body.getReader();
  // ... yield chunks
}
```

---

## Benefits of This Architecture

| Benefit | How |
|---------|-----|
| **Swap clients easily** | TUI today, Web tomorrow, same API |
| **Swap languages** | Rust CLI uses same endpoints as Python CLI |
| **Test agent independently** | Core has no I/O, easy to unit test |
| **Scale API layer** | Add load balancer, multiple instances |
| **Add auth later** | Middleware in API layer, clients unchanged |
| **Stream responses** | WebSocket/SSE for real-time TUI updates |

---

## Next Steps

1. [ ] Refactor `agent.py` → `core/agent.py` (remove CLI)
2. [ ] Add `/api/v1/` prefix to all routes
3. [ ] Add SSE streaming endpoint
4. [ ] Add WebSocket support
5. [ ] Create client library (`client/src/lib/binks_client.py`)
6. [ ] Document API with OpenAPI/Swagger
