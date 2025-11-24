# Binks - Distributed AI Infrastructure System

A decoupled, scalable AI orchestration system that separates the "Brain" (AI control plane) from the "Body" (compute cluster).

## System Architecture

```
┌──────────────────┐         ┌──────────────────┐         ┌──────────────────┐
│  Binks Client    │────────▶│  Binks           │────────▶│  Pi Cluster      │
│  (Your Laptop)   │  HTTP   │  Orchestrator    │ kubectl │  (Compute Plane) │
│                  │         │  (M3 Ultra)      │         │                  │
│  - agent.py CLI  │         │  - Ollama 405B   │         │  - K8s Master    │
│  - simple_client │         │  - Agno Agent    │         │  - Worker Nodes  │
│                  │         │  - FastAPI       │         │  - Your Apps     │
└──────────────────┘         └──────────────────┘         └──────────────────┘
     (Interface)              (AI Control Plane)            (Execution Plane)
```

## Core Philosophy

This is **not** a monolithic AI system. It's a **distributed architecture** where:

1. **The Brain (M3 Ultra)** does the heavy AI reasoning with your largest model (405B)
2. **The Body (Pi Cluster)** executes the actual work and runs your applications
3. **The Interface (Client)** provides you with a simple way to communicate with the system

The M3 Ultra is **NOT** part of the cluster - it's a **client** that manages the cluster, just like you would with `kubectl`.

## Repository Structure

```
binks/
├── README.md                    # This file
├── Makefile
├── quickstart.sh
│
├── orchestrator/                # AI Control Plane (runs on M3 Ultra)
│   └── agno/
│       ├── src/
│       │   ├── agent.py         # Master Agent + CLI
│       │   └── api/server.py    # FastAPI REST interface
│       ├── tools/
│       │   ├── kubectl_tool.py  # Cluster management
│       │   └── agent_spawner.py # Worker agent spawning
│       ├── requirements.txt
│       └── .env
│
├── client/                      # Client interface (runs on your laptop)
│   ├── src/simple_client.py
│   └── config/api-endpoints.yaml
│
├── manifests/                   # K8s manifests
│   └── k8s-manifests/
│       ├── core/
│       └── agents/
│
└── docs/                        # Documentation
    ├── ARCHITECTURE.md
    ├── ROADMAP.md
    └── PROJECT_STRUCTURE.md
```

## Quick Start

### Phase 1: Crawl (Test Locally)

**Goal**: Get the Master Agent working on your M3 Ultra

```bash
# On your M3 Ultra
cd ~/binks/orchestrator/agno

# Set up Python environment
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

# Configure environment
cp .env.example .env
# Edit .env with your settings

# Make sure Ollama is running
ollama serve  # In another terminal
ollama pull llama3.1:405b

# Run the CLI directly
python src/agent.py
```

You should see:
```
============================================================
Binks Orchestrator - Agno Implementation (Crawl Phase)
============================================================

Master Agent ready. Type your requests or 'quit' to exit.

You: What is the status of my cluster?
```

### Phase 2: Walk (Run as API)

**Goal**: Expose the Master Agent as an API

```bash
# Start the FastAPI server
python src/api/server.py
```

The API will be available at `http://<m3-ip>:8000`

From your laptop:
```bash
# Test the API
curl http://<m3-ip>:8000/health

# Send a task
curl -X POST http://<m3-ip>:8000/invoke \
  -H "Content-Type: application/json" \
  -d '{"task": "List all pods in the default namespace"}'
```

### Phase 3: Run (Full Integration)

**Goal**: Connect everything together

```bash
# On your laptop
cd ~/binks/client
pip install requests pyyaml

# Configure API endpoint
# Edit config/api-endpoints.yaml with your M3's IP

# Use interactively
python src/simple_client.py
```

## How It Works

### The Flow

1. **You** type a command in the client (e.g., "Deploy placemyparents v2.0")
2. **Client** sends an HTTP request to the Binks Orchestrator API
3. **Master Agent** (on M3) receives the task and uses **Ollama 405B** to create a plan
4. **Master Agent** executes tools:
   - `ShellTools`: Run any shell command
   - `FileTools`: Read/write files
   - `run_kubectl`: Interact with the cluster
   - `spawn_worker_agent`: Create Kubernetes Jobs for complex sub-tasks
5. **Results** flow back to the client

## Tools

The Master Agent has these capabilities:

### Pre-built Tools (Code Editing)
- **ShellTools**: Run any shell command (git, scripts, builds)
- **FileTools**: Read, write, and list files

### Custom Tools (Infrastructure)
- **run_kubectl**: Execute any kubectl command
- **get_cluster_status**: Quick cluster health check
- **spawn_worker_agent**: Create K8s Jobs for specialized agents
- **check_agent_status**: Monitor spawned workers

## Configuration

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

## Deployment

### Development (Manual)

```bash
# On M3
cd ~/binks/orchestrator/agno
source .venv/bin/activate
python src/agent.py        # CLI mode
# or
python src/api/server.py   # API mode
```

### Production (Systemd Service)

Create `/etc/systemd/system/binks.service`:

```ini
[Unit]
Description=Binks Orchestrator AI Control Plane
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

```bash
sudo systemctl enable binks
sudo systemctl start binks
```

## Troubleshooting

### Can't connect to M3 from client
```bash
ping <m3-ip>
curl http://<m3-ip>:8000/health
sudo ufw allow 8000/tcp
```

### Master Agent can't reach cluster
```bash
kubectl cluster-info
kubectl get nodes
```

### Ollama errors
```bash
curl http://localhost:11434/api/version
ollama list
ollama pull llama3.1:405b
```

## Credits

- **Agno**: Agent framework
- **Ollama**: Local LLM serving
- **Kubernetes**: Container orchestration
- **FastAPI**: Python web framework

---

**Remember**: The M3 is the CEO. The Pi cluster is the factory floor. Don't put the CEO on the factory floor.
