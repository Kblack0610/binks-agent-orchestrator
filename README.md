# Global AI - Distributed AI Infrastructure System

A decoupled, scalable AI orchestration system that separates the "Brain" (AI control plane) from the "Body" (compute cluster).

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Global AI System                        │
└─────────────────────────────────────────────────────────────────┘

┌──────────────────┐         ┌──────────────────┐         ┌──────────────────┐
│  Binks Client    │────────▶│  Binks           │────────▶│  existing cluster      │
│  (Your Laptop)   │  HTTP   │  Orchestrator    │ kubectl │  (Compute Plane) │
│                  │         │  (M3 Ultra)      │         │                  │
│  - opencode TUI  │         │  - Ollama 405B   │         │  - K8s Master    │
│  - Simple Client │         │  - CrewAI        │         │  - Worker Nodes  │
│                  │         │  - FastAPI       │         │  - Your Apps     │
└──────────────────┘         └──────────────────┘         └──────────────────┘
     (Interface)              (AI Control Plane)            (Execution Plane)
```

## Core Philosophy

This is **not** a monolithic AI system. It's a **distributed architecture** where:

1. **The Brain (M3 Ultra)** does the heavy AI reasoning with your largest model (405B)
2. **The Body (existing cluster)** executes the actual work and runs your applications
3. **The Interface (Client)** provides you with a simple way to communicate with the system

The M3 Ultra is **NOT** part of the cluster - it's a **client** that manages the cluster, just like you would with `kubectl`.

## Repository Structure

```
binks-ai-orchestrator/
├── README.md                    # This file - complete system docs
├── GETTING_STARTED.md
├── SETUP.md
├── quickstart.sh
├── Makefile
│
├── manifests/                   # K8s manifests (deploy FROM laptop TO cluster)
│   ├── README.md               # ← Read this! Not cluster setup!
│   ├── k8s-manifests/
│   │   ├── core/               # Ollama service, namespaces
│   │   ├── apps/               # Your applications
│   │   └── agents/             # Worker agent job templates
│   └── scripts/
│
├── orchestrator/                # AI Control Plane (runs on M3 Ultra)
│   ├── README.md               # ← Comparison of both implementations
│   ├── BENCHMARKING.md         # How to benchmark both
│   ├── crewai/                 # CrewAI implementation
│   │   ├── src/
│   │   ├── requirements/
│   │   └── README.md
│   └── agno/                   # Agno implementation (recommended)
│       ├── src/
│       ├── tools/
│       ├── requirements.txt
│       └── README.md
│
└── client/                      # Client interface (runs on your laptop)
    ├── README.md
    ├── src/simple_client.py
    ├── config/api-endpoints.yaml
    └── scripts/start-opencode.sh
```

## Important: About the `manifests/` Directory

⚠️ **`manifests/` is NOT cluster setup!**

- You already have a running Kubernetes cluster ✅
- You already manage it from your laptop with kubectl ✅
- `manifests/` contains **application deployments** to add AI services
- Deploy them FROM your laptop TO your existing cluster
- They run alongside your other apps (placemyparents, etc.)

See `manifests/README.md` for details.

## 🎯 Dual Implementation: CrewAI vs Agno

This project includes **two complete orchestrator implementations**:

### [CrewAI Implementation](./orchestrator/crewai/)
- **Best for**: Multi-agent collaboration, conversational workflows
- **Pros**: Rich team dynamics, extensive examples
- **Cons**: Heavier (~280MB/agent), manual API setup

### [Agno Implementation](./orchestrator/agno/) ⭐ Recommended
- **Best for**: Infrastructure orchestration, lightweight workers
- **Pros**: 10x lighter (~32MB/agent), built-in AgentOS, 8x faster startup
- **Cons**: Newer framework, less community content

**Why both?**
- Compare performance on your actual Pi cluster hardware
- Learn two leading agent frameworks
- Make data-driven decisions
- Show engineering thoughtfulness in portfolio

See `orchestrator/README.md` for detailed comparison and `orchestrator/BENCHMARKING.md` to run your own tests.

**For this project, we recommend Agno** due to its lightweight design perfect for Pi clusters.

## Quick Start

**Choose your implementation:**
- **Agno** (recommended for Pi clusters): `cd orchestrator/agno`
- **CrewAI** (for learning/comparison): `cd orchestrator/crewai`

### Phase 1: Crawl (Test Locally)

**Goal**: Get the Master Agent working on your M3 Ultra

1. **On your M3 Ultra:**

```bash
# Clone this repo
cd ~
git clone <your-repo-url> global
cd binks-ai-orchestrator/orchestrator

# Choose implementation (Agno recommended)
cd agno  # or cd crewai

# Set up Python environment
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt  # or requirements/base.txt for CrewAI

# Configure environment
cp .env.example .env
# Edit .env with your settings

# Make sure Ollama is running
ollama serve  # In another terminal
ollama pull llama3.1:405b  # Or your preferred model

# Test the agent locally
python src/main.py
```

You should see the Master Agent initialize and present an interactive prompt.

Try: `"What is the status of my cluster?"`

**Expected behavior**: The agent will use the `run_kubectl` tool to check your existing Kubernetes cluster status.

### Phase 2: Walk (Run as API)

**Goal**: Expose the Master Agent as an API

2. **On your M3 Ultra:**

```bash
cd ~/binks-ai-orchestrator/orchestrator

# Start the FastAPI server
python src/api/server.py
```

The API will be available at `http://<m3-ip>:8000`

3. **From your laptop (or any machine):**

```bash
# Test the API
curl http://<m3-ip>:8000/health

# Send a task
curl -X POST http://<m3-ip>:8000/invoke \
  -H "Content-Type: application/json" \
  -d '{"task": "List all pods in the ai-agents namespace"}'
```

### Phase 3: Run (Full Integration)

**Goal**: Connect everything together

4. **Deploy cluster services:**

```bash
# On your master pi (or from M3 with kubectl configured)
cd ~/binks-ai-orchestrator/cluster

# Create namespaces
kubectl apply -f k8s-manifests/core/namespace.yaml

# Deploy Ollama service (for worker agents)
kubectl apply -f k8s-manifests/core/ollama-deployment.yaml

# Verify
kubectl get pods -n ai-services
```

5. **Set up client on your laptop:**

```bash
cd ~/binks-ai-orchestrator/client

# Install dependencies
pip install requests pyyaml

# Configure API endpoint
# Edit config/api-endpoints.yaml with your M3's IP

# Test the simple client
python src/simple_client.py --health

# Use interactively
python src/simple_client.py
```

## How It Works

### The Flow

1. **You** type a command in the client (e.g., "Deploy placemyparents v2.0")
2. **Client** sends an HTTP request to the Binks Orchestrator API
3. **Master Agent** (on M3) receives the task and uses **Ollama 405B** to create a plan
4. **Master Agent** executes tools:
   - `run_kubectl`: To interact with the cluster directly
   - `spawn_worker_agent`: To create Kubernetes Jobs for complex sub-tasks
5. **Worker Agents** (if spawned) run on the existing cluster using the cluster's Ollama service (lightweight model)
6. **Results** flow back to the Master Agent, then to the client, then to you

### Example: Deploying an App

```
You: "Deploy the latest version of placemyparents"
  ↓
Client sends HTTP POST to M3:8000/invoke
  ↓
Master Agent (M3) thinks:
  "I need to:
   1. Check current deployment
   2. Update the image
   3. Verify the rollout"
  ↓
Master Agent executes:
  - run_kubectl("get deployment placemyparents")
  - run_kubectl("set image deployment/placemyparents ...")
  - spawn_worker_agent("deployment-verifier", {...})
  ↓
Worker Agent (K8s Job) starts on existing cluster:
  - Waits for pods to be ready
  - Runs health checks
  - Reports back
  ↓
You receive: "✓ Deployment complete! All pods healthy."
```

## Prerequisites

### Hardware
- **M3 Ultra** (or any powerful machine) for the orchestrator
- **Kubernetes cluster** (your existing cluster + other nodes)
- **Client machine** (laptop, desktop, etc.)

### Software
- **On M3 Ultra:**
  - Python 3.11+
  - Ollama
  - kubectl (configured to access your existing Kubernetes cluster)

- **On existing cluster:**
  - Kubernetes (already set up)
  - Network access from M3

- **On Client:**
  - Python 3.11+ (for simple_client.py)
  - OR opencode (optional)
  - Network access to M3

## Configuration

### Connecting M3 to Cluster

The M3 needs `kubectl` access to your existing Kubernetes cluster:

```bash
# On your master pi
cat ~/.kube/config

# Copy the output

# On your M3
mkdir -p ~/.kube
nano ~/.kube/config
# Paste the config

# Test
kubectl get nodes
```

### Environment Variables

On your M3, edit `orchestrator/.env`:

```bash
# Ollama (running locally on M3)
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=llama3.1:405b

# Kubernetes
KUBECONFIG_PATH=~/.kube/config

# API Server
API_HOST=0.0.0.0
API_PORT=8000
```

## The "Crawl, Walk, Run" Approach

We built this system incrementally:

| Phase | Goal | What Works |
|-------|------|------------|
| **Crawl** | Basic agent | Master Agent runs locally on M3, can execute kubectl commands |
| **Walk** | API exposure | Master Agent exposed via FastAPI, can be called remotely |
| **Run** | Full system | Client connects to M3, M3 spawns worker agents on cluster |

You can stop at any phase depending on your needs.

## Tools

The Master Agent has these "hands":

### 1. `run_kubectl`
Execute any kubectl command on the cluster.

```python
run_kubectl("get pods -n ai-agents")
run_kubectl("describe deployment placemyparents")
```

### 2. `get_cluster_status`
Quick health check of the cluster (nodes + pods).

### 3. `spawn_worker_agent`
Create a Kubernetes Job to run a specialized agent.

```python
spawn_worker_agent(
    agent_type="code-reviewer",
    task_params={"repo_url": "...", "branch": "main"}
)
```

### 4. `check_agent_status`
Monitor a spawned worker agent's progress.

## Worker Agents

Worker agents are **Kubernetes Jobs** defined in `manifests/k8s-manifests/agents/`.

Currently available:
- `code-reviewer-job.yaml`: Reviews code changes

To add a new worker agent:
1. Create `manifests/k8s-manifests/agents/<agent-name>-job.yaml`
2. Define the agent's script in a ConfigMap
3. The Master Agent can now spawn it with `spawn_worker_agent(agent_type="<agent-name>")`

## Deployment

### Development (Manual)

```bash
# 1. Update code locally
git add .
git commit -m "Update orchestrator"
git push

# 2. Pull on M3
ssh user@m3
cd ~/global
git pull

# 3. Restart orchestrator
cd orchestrator
source venv/bin/activate
python src/api/server.py
```

### Production (Systemd Service)

Create `/etc/systemd/system/orchestrator.service`:

```ini
[Unit]
Description=Binks Orchestrator AI Control Plane
After=network.target

[Service]
Type=simple
User=your-user
WorkingDirectory=/home/your-user/binks-ai-orchestrator/orchestrator
Environment="PATH=/home/your-user/binks-ai-orchestrator/orchestrator/venv/bin"
ExecStart=/home/your-user/binks-ai-orchestrator/orchestrator/venv/bin/python src/api/server.py
Restart=always

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable orchestrator
sudo systemctl start orchestrator
sudo systemctl status orchestrator
```

## Monitoring

### Check Orchestrator Status
```bash
curl http://<m3-ip>:8000/health
```

### View Logs
```bash
# If running manually
tail -f logs/orchestrator.log

# If running as systemd service
sudo journalctl -u orchestrator -f
```

### Monitor Cluster
```bash
kubectl get pods -n ai-agents  # Worker agents
kubectl get pods -n ai-services  # Ollama service
kubectl top nodes  # Resource usage
```

## Troubleshooting

### Can't connect to M3 from client
```bash
# Check network
ping <m3-ip>

# Check if API is running
curl http://<m3-ip>:8000/health

# Check firewall on M3
sudo ufw status
sudo ufw allow 8000/tcp
```

### Master Agent can't reach cluster
```bash
# On M3, test kubectl
kubectl cluster-info
kubectl get nodes

# Check kubeconfig
cat ~/.kube/config

# Test network to master pi
ping <master-pi-ip>
```

### Ollama errors
```bash
# Check if Ollama is running
curl http://localhost:11434/api/version

# Check available models
ollama list

# Pull model if missing
ollama pull llama3.1:405b
```

### Worker agents not spawning
```bash
# Check if namespaces exist
kubectl get namespaces

# Check if Ollama service is running
kubectl get pods -n ai-services

# Check job template exists
ls -la manifests/k8s-manifests/agents/
```

## Extending the System

### Adding a New Tool

1. Create `orchestrator/src/tools/my_tool.py`:

```python
from crewai.tools import tool

@tool("my_tool")
def my_tool(param: str) -> str:
    """Description of what this tool does."""
    # Your implementation
    return "result"
```

2. Import in `orchestrator/src/agents/master_agent.py`:

```python
from tools.my_tool import my_tool

# Add to agent's tools list
tools=[..., my_tool]
```

### Adding a New Worker Agent

1. Create `manifests/k8s-manifests/agents/my-agent-job.yaml`
2. Define the agent's script and dependencies
3. Use `spawn_worker_agent(agent_type="my-agent", ...)`

### Adding a New Application

1. Create `manifests/k8s-manifests/apps/my-app/`
2. Add deployment, service, ingress manifests
3. Deploy with: `kubectl apply -f manifests/k8s-manifests/apps/my-app/`

## Security Considerations

1. **API Authentication**: The FastAPI server currently has no authentication. Add API keys or OAuth in production.
2. **Network Security**: Use firewalls and VPNs to restrict access to the M3 API.
3. **Kubectl Access**: The M3 has full cluster access. Protect the kubeconfig file.
4. **Worker Agents**: Worker agents run arbitrary code. Only spawn trusted agents.

## Performance Tips

1. **Model Selection**: Use 405B for complex planning, but consider 70B or smaller for faster responses
2. **Worker Agents**: Offload long-running tasks to worker agents to keep the Master Agent responsive
3. **Cluster Resources**: Monitor your existing cluster's resource usage and scale appropriately
4. **Caching**: Consider adding response caching in the FastAPI layer for common queries

## Next Steps

- [ ] Set up Ollama on M3 with your preferred model
- [ ] Configure kubectl access from M3 to cluster
- [ ] Test Master Agent locally (Crawl phase)
- [ ] Deploy FastAPI server (Walk phase)
- [ ] Set up client and test full integration (Run phase)
- [ ] Deploy your first application via the agent
- [ ] Create custom worker agents for your specific needs
- [ ] Add authentication to the API
- [ ] Set up monitoring and logging
- [ ] Build custom workflows

## Contributing

This is your personal infrastructure, but if you want to share improvements:
1. Fork the repo
2. Create a feature branch
3. Test thoroughly on your setup
4. Submit a pull request

## License

(Add your preferred license)

## Credits

- **CrewAI**: Multi-agent framework
- **Ollama**: Local LLM serving
- **Kubernetes**: Container orchestration
- **FastAPI**: Modern Python web framework

---

**Remember**: The M3 is the CEO. The existing cluster is the factory floor. Don't put the CEO on the factory floor.
