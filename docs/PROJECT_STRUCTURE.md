# Global AI - Project Structure

## Quick Reference

```
global/
│
├── README.md                    # Main documentation
├── SETUP.md                     # Step-by-step setup guide
├── PROJECT_STRUCTURE.md         # This file
├── quickstart.sh                # Interactive setup script
├── .gitignore                   # Git ignore rules
│
├── cluster/                     # Pi Cluster (Kubernetes Compute Plane)
│   ├── README.md
│   ├── k8s-manifests/
│   │   ├── core/
│   │   │   ├── namespace.yaml           # ai-services, ai-agents namespaces
│   │   │   └── ollama-deployment.yaml   # Ollama service for worker agents
│   │   ├── apps/                        # Your applications go here
│   │   └── agents/
│   │       └── code-reviewer-job.yaml   # Example worker agent template
│   ├── docs/
│   └── scripts/
│
├── orchestrator/          # M3 Ultra (AI Control Plane)
│   ├── README.md
│   ├── .env.example             # Configuration template
│   ├── src/
│   │   ├── main.py              # Entry point for local testing
│   │   ├── agents/
│   │   │   └── master_agent.py  # The main orchestrator agent
│   │   ├── tools/
│   │   │   ├── kubectl_tool.py      # Interact with cluster
│   │   │   └── agent_spawner.py     # Spawn worker agents
│   │   └── api/
│   │       └── server.py        # FastAPI server
│   ├── config/
│   ├── requirements/
│   │   └── base.txt             # Python dependencies
│   └── tests/
│
└── client/                # Client Interface (Laptop/Desktop)
    ├── README.md
    ├── src/
    │   └── simple_client.py     # Python CLI client
    ├── config/
    │   └── api-endpoints.yaml   # M3 API endpoint configuration
    └── scripts/
        └── start-opencode.sh    # Launch opencode TUI
```

## File Purposes

### Root Level

| File | Purpose |
|------|---------|
| README.md | Complete system documentation and architecture overview |
| SETUP.md | Step-by-step setup instructions for all components |
| quickstart.sh | Interactive script to help set up each component |
| .gitignore | Prevents committing sensitive files (env, logs, etc.) |

### Cluster (Kubernetes)

| File | Purpose | Deploy To |
|------|---------|-----------|
| cluster/README.md | Cluster-specific documentation | Master Pi |
| namespace.yaml | Creates ai-services and ai-agents namespaces | K8s Cluster |
| ollama-deployment.yaml | Deploys Ollama service for worker agents | K8s Cluster |
| code-reviewer-job.yaml | Template for code review worker agent | Used by M3 |

### Binks Orchestrator (M3)

| File | Purpose | Runs On |
|------|---------|---------|
| orchestrator/README.md | Orchestrator documentation | - |
| .env.example | Configuration template | M3 |
| main.py | Entry point for testing agent locally | M3 |
| master_agent.py | The "Brain" - main orchestrator agent | M3 |
| kubectl_tool.py | Tool for running kubectl commands | M3 |
| agent_spawner.py | Tool for spawning worker agents | M3 |
| server.py | FastAPI REST API server | M3 |
| base.txt | Python package dependencies | M3 |

### Binks Client (Laptop)

| File | Purpose | Runs On |
|------|---------|---------|
| client/README.md | Client documentation | - |
| simple_client.py | Python CLI client (alternative to opencode) | Client |
| api-endpoints.yaml | M3 API endpoint configuration | Client |
| start-opencode.sh | Script to launch opencode TUI | Client |

## Key Concepts

### The Three Planes

1. **AI Control Plane** (M3 Ultra / orchestrator)
   - Runs the "Brain" (405B LLM via Ollama)
   - Executes CrewAI Master Agent
   - Plans and orchestrates all tasks
   - Manages cluster via kubectl
   - Exposes REST API

2. **Compute Plane** (Pi Cluster / cluster)
   - Executes actual workloads
   - Runs your applications
   - Runs worker agents (spawned by M3)
   - Provides lightweight Ollama service for workers

3. **Interface Plane** (Laptop / client)
   - Your "window" into the system
   - Sends tasks to M3 via HTTP
   - Can use simple_client.py or opencode

### The Flow

```
You (Client)
    ↓
  HTTP POST to M3:8000/invoke
    ↓
Master Agent (M3)
    ├─→ Uses Ollama (405B) to plan
    ├─→ Executes kubectl commands directly
    └─→ Spawns worker agents as K8s Jobs
            ↓
        Worker Agents (Pi Cluster)
            ├─→ Use cluster's Ollama (8B)
            └─→ Report results back
```

### The Tools

The Master Agent has four "hands":

1. **run_kubectl** - Execute kubectl commands on cluster
2. **get_cluster_status** - Quick health check
3. **spawn_worker_agent** - Create K8s Job for specialized tasks
4. **check_agent_status** - Monitor spawned worker agents

## Getting Started

### Quick Setup

```bash
# 1. Choose your component and run quickstart
./quickstart.sh

# 2. Follow the prompts based on which machine you're on:
#    - Option 1: M3 Orchestrator
#    - Option 2: Pi Cluster
#    - Option 3: Client
```

### Manual Setup

See SETUP.md for detailed step-by-step instructions.

## Deployment Order

1. **Cluster** - Set up K8s services first
2. **Orchestrator** - Set up M3 with Ollama and agent
3. **Client** - Set up client and test end-to-end

## Development Workflow

```
Local Machine (where this repo is)
    ↓
  Edit files
    ↓
  git commit & push
    ↓
  ┌──────────────┬──────────────┬──────────────┐
  ↓              ↓              ↓              ↓
Master Pi      M3 Ultra       Client       Other Pis
  ↓              ↓              ↓
git pull      git pull      git pull
  ↓              ↓              ↓
kubectl       restart       restart
apply         service       client
```

## Configuration Files

### Environment Variables (.env on M3)

```bash
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=llama3.1:405b
KUBECONFIG_PATH=~/.kube/config
API_HOST=0.0.0.0
API_PORT=8000
```

### API Endpoints (client/config/api-endpoints.yaml)

```yaml
default: local
environments:
  local:
    host: "192.168.1.XXX"  # Your M3 IP
    port: 8000
    protocol: "http"
```

## Common Commands

### On M3 Orchestrator

```bash
# Test locally
cd ~/global/orchestrator
source venv/bin/activate
python src/main.py

# Run API server
python src/api/server.py

# Check cluster access
kubectl get nodes
```

### On Master Pi

```bash
# Deploy cluster services
cd ~/global/cluster
kubectl apply -f k8s-manifests/core/

# Check status
kubectl get pods -n ai-services
kubectl get jobs -n ai-agents
```

### On Client

```bash
# Test connection
python src/simple_client.py --health

# Get cluster status
python src/simple_client.py --cluster

# Interactive mode
python src/simple_client.py
```

## Port Reference

| Service | Machine | Port | Access |
|---------|---------|------|--------|
| K8s API | Master Pi | 6443 | M3 needs access |
| Ollama (M3) | M3 Ultra | 11434 | Local only |
| Ollama (Cluster) | Pi Cluster | 11434 | Cluster internal |
| FastAPI | M3 Ultra | 8000 | Client needs access |

## Next Steps

After initial setup:

1. Test basic commands
2. Deploy your first application
3. Create custom worker agents
4. Add authentication to API
5. Set up monitoring
6. Build custom workflows

## Support

- Main docs: README.md
- Setup guide: SETUP.md
- Component-specific: See README.md in each subdirectory
