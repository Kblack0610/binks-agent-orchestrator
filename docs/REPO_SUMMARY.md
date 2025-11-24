# Binks AI Orchestrator - Repository Summary

## Structure

```
binks/
│
├── README.md                             # System documentation
├── Makefile                              # Common operations
├── quickstart.sh                         # Interactive setup
├── .gitignore
│
├── orchestrator/                         # AI Control Plane (M3 Ultra)
│   └── agno/
│       ├── src/
│       │   ├── agent.py                  # Master Agent + CLI
│       │   └── api/server.py             # FastAPI REST interface
│       ├── tools/
│       │   ├── kubectl_tool.py           # Cluster management
│       │   └── agent_spawner.py          # Worker agent spawning
│       ├── requirements.txt
│       └── .env
│
├── client/                               # Client Interface
│   ├── src/
│   │   └── simple_client.py              # Python CLI client
│   └── config/
│       └── api-endpoints.yaml            # API endpoint config
│
├── manifests/                            # Kubernetes Manifests
│   └── k8s-manifests/
│       └── core/
│
└── docs/                                 # Documentation
    ├── ARCHITECTURE.md
    ├── ROADMAP.md
    ├── PROJECT_STRUCTURE.md
    └── REPO_SUMMARY.md                   # This file
```

## Components

| Component | Location | Purpose |
|-----------|----------|---------|
| Agent CLI | `orchestrator/agno/src/agent.py` | Interactive CLI for direct use |
| API Server | `orchestrator/agno/src/api/server.py` | REST API for remote clients |
| Python Client | `client/src/simple_client.py` | Remote CLI client |
| K8s Tools | `orchestrator/agno/tools/` | Kubectl + Agent spawning |

## Key Features

- Agno-powered master agent with tool orchestration
- Ollama integration for local LLM inference (supports 405B models)
- Pre-built tools (ShellTools, FileTools) for code editing
- Custom tools (KubectlToolkit, AgentSpawnerToolkit) for infrastructure
- REST API interface (FastAPI)
- Natural language infrastructure management

## Tech Stack

```
Agent Framework: Agno
Backend: Python, FastAPI
LLM: Ollama (Llama 3.1)
Infrastructure: Kubernetes
```

## Quick Start

```bash
# Run the CLI directly
cd orchestrator/agno
python src/agent.py

# Or run the API server
python src/api/server.py

# Then use the client
cd client
python src/simple_client.py
```

## Architecture

```
┌──────────────────┐     ┌──────────────────┐
│   agent.py CLI   │     │  simple_client   │
│   (direct use)   │     │   (via HTTP)     │
└────────┬─────────┘     └────────┬─────────┘
         │                        │
         │ Direct                 │ HTTP
         │                        │
         ▼                        ▼
┌─────────────────────────────────────────────┐
│              Agno Master Agent              │
│                                             │
│  ┌─────────────┐    ┌────────────────────┐ │
│  │ Pre-built   │    │ Custom             │ │
│  │ ShellTools  │    │ KubectlToolkit     │ │
│  │ FileTools   │    │ AgentSpawnerToolkit│ │
│  └─────────────┘    └────────────────────┘ │
│                                             │
│              Ollama (LLM)                   │
└─────────────────────────────────────────────┘
```
