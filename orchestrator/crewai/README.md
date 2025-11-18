# Binks Orchestrator - AI Control Plane

This is your **M3 Ultra's AI Control Plane** - the "Brain" of your Global AI system.

## What Runs Here

On your M3 Ultra, you will run:

1. **Ollama Server** - Serving your largest model (e.g., 405B Llama 3.1)
2. **CrewAI Master Agent** - The orchestrator that plans and delegates tasks
3. **FastAPI Server** - Exposes the agent as an API for the client to call

## Architecture

```
┌─────────────────────────────────────────────────┐
│            M3 Ultra (Binks Orchestrator)        │
│                                                 │
│  ┌─────────────┐      ┌──────────────────────┐ │
│  │   Ollama    │◄─────│  CrewAI Master Agent │ │
│  │   Server    │      │                      │ │
│  │  (405B LLM) │      │  - Planning          │ │
│  │             │      │  - Tool Execution    │ │
│  └─────────────┘      │  - Agent Spawning    │ │
│                       └──────┬───────────────┘ │
│                              │                  │
│                       ┌──────▼───────────────┐ │
│                       │   FastAPI Server     │ │
│                       │  (Port 8000)         │ │
│                       └──────┬───────────────┘ │
└──────────────────────────────┼──────────────────┘
                               │
                 ┌─────────────┼──────────────┐
                 │             │              │
         ┌───────▼──────┐      │      ┌───────▼─────────┐
         │  Binks Client│      │      │   Pi Cluster    │
         │  (opencode)  │      │      │  (kubectl API)  │
         └──────────────┘      │      └─────────────────┘
                               │
                        (Network Connection)
```

## Directory Structure

```
binks-orchestrator/
├── src/
│   ├── agents/         # Agent definitions (Master Agent, etc.)
│   ├── tools/          # Custom tools (run_kubectl, etc.)
│   ├── api/            # FastAPI server
│   └── main.py         # Entry point
├── config/             # Configuration files
├── tests/              # Unit tests
├── requirements/       # Python dependencies
├── .env.example        # Environment variables template
└── README.md          # This file
```

## Prerequisites

### 1. Ollama Setup (on M3)

```bash
# Install Ollama (if not already installed)
brew install ollama

# Pull your largest model
ollama pull llama3.1:405b  # Or your preferred large model

# Start Ollama server
ollama serve
```

Ollama will run on `http://localhost:11434`

### 2. Python Environment

```bash
# Navigate to this directory
cd binks-orchestrator

# Create virtual environment
python3 -m venv venv
source venv/bin/activate

# Install dependencies
pip install -r requirements/base.txt
```

### 3. Kubectl Access to Cluster

The orchestrator needs to manage your Pi Cluster. Copy kubeconfig from your master pi:

```bash
# On your master pi
cat ~/.kube/config

# On your M3, save to
~/.kube/config

# Test connection
kubectl get nodes
```

### 4. Environment Configuration

```bash
cp .env.example .env
# Edit .env with your settings
```

## Quick Start

### Phase 1: Test the Agent Locally (Crawl)

Run the basic agent directly:

```bash
python src/main.py
```

This runs a simple CLI version of the agent for testing.

### Phase 2: Run as API Server (Walk)

Start the FastAPI server:

```bash
python src/api/server.py
```

Test with curl:

```bash
curl -X POST http://localhost:8000/invoke \
  -H "Content-Type: application/json" \
  -d '{"task": "List all pods in the cluster"}'
```

### Phase 3: Full Integration with Client (Run)

Once the API is running, configure your `binks-client` to connect to:
- `http://<m3-ip>:8000`

## The Master Agent

The Master Agent has these capabilities (tools):

1. **run_kubectl** - Execute kubectl commands on the Pi Cluster
2. **spawn_agent** - Create a new worker agent as a K8s Job
3. **query_cluster_status** - Check health of cluster resources
4. **manage_deployments** - Deploy/update applications

### Example Workflow

When you ask: "Deploy the latest version of placemyparents"

1. **Master Agent** uses Ollama to create a plan:
   - "I need to check the current deployment status"
   - "I need to pull the latest image"
   - "I need to update the deployment"

2. **Master Agent** executes tools:
   - Calls `run_kubectl("get deployment placemyparents")`
   - Calls `run_kubectl("set image deployment/placemyparents ...")`

3. **Master Agent** may spawn a worker:
   - Calls `spawn_agent("deployment-verifier")` to create a K8s Job
   - This job runs on the cluster and verifies the deployment
   - Results are reported back

## Configuration

### Ollama Connection
```python
# In config/settings.py
OLLAMA_BASE_URL = "http://localhost:11434"
OLLAMA_MODEL = "llama3.1:405b"
```

### Cluster Connection
```python
KUBECONFIG_PATH = "~/.kube/config"
CLUSTER_CONTEXT = "your-cluster-context"
```

### API Server
```python
API_HOST = "0.0.0.0"
API_PORT = 8000
```

## Tools

### run_kubectl

```python
from tools.kubectl_tool import run_kubectl

# The agent can call this
result = run_kubectl("get pods -n ai-agents")
```

### spawn_agent

```python
from tools.agent_spawner import spawn_agent

# Spawn a code reviewer
job_name = spawn_agent(
    agent_type="code-reviewer",
    params={
        "repo_url": "https://github.com/user/repo",
        "task_id": "123"
    }
)
```

## Deployment to M3

From your dev machine (where this repo is):

```bash
# 1. Commit your changes
git add .
git commit -m "Update orchestrator config"
git push

# 2. SSH to your M3
ssh user@m3-ultra.local

# 3. Pull the latest code
cd ~/global/binks-orchestrator
git pull

# 4. Restart the service (if running as systemd service)
sudo systemctl restart binks-orchestrator

# Or just run it directly
python src/api/server.py
```

## Running as a Service

Create a systemd service on your M3 (optional but recommended):

```bash
# Create service file (see docs/systemd-service.md)
sudo systemctl enable binks-orchestrator
sudo systemctl start binks-orchestrator
sudo systemctl status binks-orchestrator
```

## Monitoring

Check logs:
```bash
# If running directly
tail -f logs/orchestrator.log

# If running as service
sudo journalctl -u binks-orchestrator -f
```

## Development Workflow

```
┌──────────────────────┐
│  Code on dev machine │
│  (this repo)         │
└──────┬───────────────┘
       │
       │ git push
       ▼
┌──────────────────────┐
│  GitHub/GitLab       │
│  (source of truth)   │
└──────┬───────────────┘
       │
       │ git pull
       ▼
┌──────────────────────┐
│  M3 Ultra            │
│  (deployment)        │
└──────────────────────┘
```

## Testing

```bash
# Run unit tests
pytest tests/

# Test individual components
python -m pytest tests/test_tools.py -v
```

## Troubleshooting

### Can't connect to Ollama
```bash
# Check if Ollama is running
curl http://localhost:11434/api/version

# Start Ollama
ollama serve
```

### Can't connect to cluster
```bash
# Test kubectl
kubectl cluster-info

# Check kubeconfig
cat ~/.kube/config

# Test network to master pi
ping <master-pi-ip>
```

### Agent errors
```bash
# Check logs
tail -f logs/orchestrator.log

# Run in debug mode
python src/main.py --debug
```

## Next Steps

1. [ ] Set up Ollama on M3 with large model
2. [ ] Configure kubectl access to cluster
3. [ ] Test basic agent locally
4. [ ] Deploy FastAPI server
5. [ ] Connect binks-client
