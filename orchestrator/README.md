# Binks Orchestrator

The AI Control Plane that runs on your M3 Ultra.

## Overview

The orchestrator is the "brain" of the Binks system. It:
- Runs the Agno Master Agent
- Processes natural language commands
- Executes tools (shell, files, kubectl)
- Spawns worker agents on your K8s cluster

## Structure

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
    ├── requirements.txt
    └── .env
```

## Quick Start

```bash
cd orchestrator/agno

# Set up environment
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

# Configure
cp .env.example .env
# Edit .env with your Ollama settings

# Run CLI
python src/agent.py

# Or run API server
python src/api/server.py
```

## Tools

### Pre-built (Agno)
- **ShellTools**: Run any shell command
- **FileTools**: Read/write files

### Custom
- **KubectlToolkit**: Execute kubectl commands
- **AgentSpawnerToolkit**: Spawn K8s Jobs for worker agents

## Configuration

Edit `.env`:

```bash
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=llama3.1:405b
AGNO_API_HOST=0.0.0.0
AGNO_API_PORT=8000
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/invoke` | POST | Send task to agent |
| `/cluster/status` | POST | Get K8s cluster status |
| `/agent/info` | GET | Agent information |

## Usage

### CLI Mode
```bash
python src/agent.py
```

```
You: Check cluster status
Agent: All 4 nodes Ready. 15 pods running.
```

### API Mode
```bash
python src/api/server.py
```

```bash
curl -X POST http://localhost:8000/invoke \
  -H "Content-Type: application/json" \
  -d '{"task": "List all pods"}'
```
