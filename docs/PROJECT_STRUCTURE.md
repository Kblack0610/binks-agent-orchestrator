# Binks - Project Structure

## Quick Reference

```
binks/
│
├── README.md                    # Main documentation
├── Makefile                     # Build/run commands
├── quickstart.sh                # Interactive setup script
├── .gitignore                   # Git ignore rules
│
├── orchestrator/                # M3 Ultra (AI Control Plane)
│   └── agno/
│       ├── src/
│       │   ├── agent.py         # Master Agent + CLI
│       │   └── api/
│       │       └── server.py    # FastAPI server
│       ├── tools/
│       │   ├── kubectl_tool.py  # Kubernetes interactions
│       │   └── agent_spawner.py # Spawn worker agents
│       ├── requirements.txt
│       └── .env
│
├── client/                      # Client Interface
│   ├── src/
│   │   └── simple_client.py     # Python CLI client
│   └── config/
│       └── api-endpoints.yaml   # API configuration
│
├── manifests/                   # Kubernetes manifests
│   └── k8s-manifests/
│       └── core/
│
└── docs/                        # Documentation
    ├── ARCHITECTURE.md
    ├── ROADMAP.md
    └── PROJECT_STRUCTURE.md     # This file
```

## Components

### Orchestrator (M3 Ultra)

The "brain" of the system. Runs the Agno Master Agent.

| File | Purpose |
|------|---------|
| `agent.py` | Master Agent + interactive CLI |
| `server.py` | FastAPI REST API |
| `kubectl_tool.py` | Kubernetes command execution |
| `agent_spawner.py` | Spawn worker agents as K8s Jobs |

**Run the CLI:**
```bash
cd orchestrator/agno
source .venv/bin/activate
python src/agent.py
```

**Run the API server:**
```bash
python src/api/server.py
```

### Client

Python client for remote access to the orchestrator.

| File | Purpose |
|------|---------|
| `simple_client.py` | CLI client that talks to API |
| `api-endpoints.yaml` | Server connection config |

**Run the client:**
```bash
cd client
python src/simple_client.py
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     CLIENTS                              │
│                                                          │
│   agent.py CLI     simple_client.py    Future: Web/Rust │
│        │                  │                   │          │
└────────┼──────────────────┼───────────────────┼──────────┘
         │                  │                   │
         │ Direct      HTTP │              HTTP │
         │                  │                   │
         ▼                  ▼                   ▼
┌─────────────────────────────────────────────────────────┐
│                    ORCHESTRATOR                          │
│                                                          │
│   ┌─────────────────────────────────────────────────┐   │
│   │              FastAPI Server                      │   │
│   │         POST /invoke  GET /health               │   │
│   └─────────────────────┬───────────────────────────┘   │
│                         │                                │
│   ┌─────────────────────▼───────────────────────────┐   │
│   │              Agno Master Agent                   │   │
│   │                                                  │   │
│   │  Tools:                                          │   │
│   │  ├── ShellTools (pre-built)                     │   │
│   │  ├── FileTools (pre-built)                      │   │
│   │  ├── KubectlToolkit (custom)                    │   │
│   │  └── AgentSpawnerToolkit (custom)               │   │
│   └─────────────────────┬───────────────────────────┘   │
│                         │                                │
│   ┌─────────────────────▼───────────────────────────┐   │
│   │              Ollama (LLM)                        │   │
│   │              llama3.1:405b                       │   │
│   └─────────────────────────────────────────────────┘   │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

## Two Ways to Use

### 1. Direct CLI (Local)

Run `agent.py` directly on the M3:

```bash
python src/agent.py
```

```
============================================================
Binks Orchestrator - Agno Implementation
============================================================

Master Agent ready. Type your requests or 'quit' to exit.

You: Check cluster status
```

### 2. Remote API (Network)

Run the server, then use any client:

**Server (M3):**
```bash
python src/api/server.py
```

**Client (Laptop):**
```bash
python src/simple_client.py
# Or: curl http://m3-ip:8000/invoke -d '{"task": "..."}'
```

## Environment Setup

### M3 Orchestrator (.env)

```bash
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=llama3.1:405b
AGNO_API_HOST=0.0.0.0
AGNO_API_PORT=8000
```

### Client (api-endpoints.yaml)

```yaml
default: local
environments:
  local:
    host: "192.168.1.XXX"
    port: 8000
    protocol: "http"
```

## Common Commands

```bash
# On M3 - Run CLI directly
cd orchestrator/agno && python src/agent.py

# On M3 - Run API server
cd orchestrator/agno && python src/api/server.py

# On Client - Interactive mode
cd client && python src/simple_client.py

# On Client - Single command
python src/simple_client.py "Check cluster health"

# On Client - Health check
python src/simple_client.py --health
```

## Port Reference

| Service | Machine | Port |
|---------|---------|------|
| Ollama | M3 Ultra | 11434 |
| FastAPI | M3 Ultra | 8000 |
| K8s API | Master Pi | 6443 |
