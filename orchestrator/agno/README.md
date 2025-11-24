# Binks Orchestrator - Agno Implementation

The main orchestrator using **Agno** - optimized for **lightweight, high-performance infrastructure orchestration**.

## Features

- **Lightweight** - Minimal dependencies, small Docker images
- **Fast** - Quick startup times
- **Built-in API** - FastAPI server included
- **Infrastructure-first** - Designed for orchestration tasks

## Quick Start

### Prerequisites

```bash
# Install Ollama
brew install ollama

# Pull your model
ollama pull llama3.1:405b  # Or smaller: llama3.1:8b

# Start Ollama
ollama serve
```

### Installation

```bash
cd orchestrator/agno

# Create virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install dependencies
pip install -r requirements.txt

# Configure
cp .env.example .env
# Edit .env with your settings
```

### Run CLI

```bash
python src/agent.py
```

Try:
- "What is the status of my cluster?"
- "List all pods"
- "Spawn a code-reviewer agent"

### Run API Server

```bash
python src/api/server.py
```

Access at http://localhost:8000

Test:
```bash
curl -X POST http://localhost:8000/invoke \
  -H "Content-Type: application/json" \
  -d '{"task": "Get cluster status"}'
```

## Architecture

```
┌────────────────────────────────────────┐
│           FastAPI Server               │
│         POST /invoke                   │
└──────────────────┬─────────────────────┘
                   │
┌──────────────────▼─────────────────────┐
│           Master Agent                  │
│                                         │
│  Tools:                                 │
│  ├── ShellTools (pre-built)            │
│  ├── FileTools (pre-built)             │
│  ├── KubectlToolkit (custom)           │
│  └── AgentSpawnerToolkit (custom)      │
│                                         │
│              Ollama (LLM)               │
└─────────────────────────────────────────┘
```

## Tools

### Pre-built (Agno)
- **ShellTools** - Run any shell command
- **FileTools** - Read/write files

### Custom
- **KubectlToolkit**
  - `run_kubectl(command, namespace)` - Execute kubectl
  - `get_cluster_status()` - Quick health check

- **AgentSpawnerToolkit**
  - `spawn_worker_agent(agent_type, task_params)` - Create K8s Job
  - `check_agent_status(job_name)` - Monitor worker

## Configuration

Edit `.env`:

```bash
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=llama3.1:405b
AGNO_API_HOST=0.0.0.0
AGNO_API_PORT=8000
```

## Project Structure

```
agno/
├── src/
│   ├── agent.py          # Master Agent + CLI
│   ├── api/
│   │   └── server.py     # FastAPI server
│   └── playground.py     # AgentOS playground
├── tools/
│   ├── kubectl_tool.py   # Kubectl toolkit
│   └── agent_spawner.py  # Agent spawning toolkit
├── requirements.txt
├── .env.example
└── README.md
```

## Deployment

### Development

```bash
# CLI mode
python src/agent.py

# API mode
python src/api/server.py
```

### Production (Systemd)

```ini
[Unit]
Description=Binks Orchestrator
After=network.target

[Service]
Type=simple
User=your-user
WorkingDirectory=/home/your-user/binks/orchestrator/agno
Environment="PATH=/home/your-user/binks/orchestrator/agno/.venv/bin"
ExecStart=/home/your-user/binks/orchestrator/agno/.venv/bin/python src/api/server.py
Restart=always

[Install]
WantedBy=multi-user.target
```

## Troubleshooting

### Can't connect to Ollama

```bash
curl http://localhost:11434/api/version
ollama serve
```

### Kubectl fails

```bash
kubectl get nodes
cat ~/.kube/config
```

## Resources

- [Agno Documentation](https://docs.agno.com)
- [Agno GitHub](https://github.com/agno-agi/agno)
