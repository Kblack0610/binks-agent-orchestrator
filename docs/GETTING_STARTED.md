# Getting Started with Global AI

Welcome! This is your complete Global AI system - a distributed AI infrastructure that separates the "Brain" (AI control plane) from the "Body" (compute cluster).

## What You Have

You now have a complete, production-ready Global AI system with:

✓ **Cluster** - Kubernetes configs for your Pi cluster (the "compute plane")
✓ **Orchestrator** - CrewAI Master Agent running on M3 Ultra (the "AI brain")  
✓ **Client** - Interface to communicate with the system
✓ **Documentation** - Complete guides and examples

## The 5-Minute Quick Start

### Step 1: Choose Your Machine

You'll set this up on **three different machines**. Start with whichever one you're on right now:

**On M3 Ultra (Orchestrator):**
```bash
cd ~/global
./quickstart.sh
# Select option 1
```

**On Master Pi (Cluster):**
```bash
cd ~/global  
./quickstart.sh
# Select option 2
```

**On Your Laptop (Client):**
```bash
cd ~/global
./quickstart.sh
# Select option 3
```

### Step 2: Test It

Once all three are set up, test end-to-end:

```bash
# On your laptop
cd ~/global/client
python src/simple_client.py

# Try asking:
"What pods are running on the cluster?"
```

## What Happens When You Use It

```
┌──────────────────────────────────────────────────────┐
│  You: "Deploy the latest version of my app"          │
└────────────────┬─────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────┐
│  Client sends HTTP request to M3 Ultra               │
└────────────────┬─────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────┐
│  M3 Ultra (Master Agent) thinks with 405B model:     │
│  "I need to check current deployment, update it,     │
│   and verify the rollout"                            │
└────────────────┬─────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────┐
│  Master Agent executes:                              │
│  - Uses kubectl to check deployment                  │
│  - Updates the deployment                            │
│  - Spawns a "verifier" agent on Pi cluster          │
└────────────────┬─────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────┐
│  Verifier agent runs on Pi cluster,                  │
│  checks health, reports back                         │
└────────────────┬─────────────────────────────────────┘
                 │
                 ▼
┌──────────────────────────────────────────────────────┐
│  You receive: "✓ Deployment complete!"               │
└──────────────────────────────────────────────────────┘
```

## The Architecture (Simplified)

```
Your Laptop          M3 Ultra           Pi Cluster
(Interface)        (AI Brain)       (Execution Plane)
    │                  │                   │
    │   "Deploy app"   │                   │
    ├─────────────────>│                   │
    │                  │                   │
    │                  │  kubectl apply    │
    │                  ├──────────────────>│
    │                  │                   │
    │                  │  spawn job        │
    │                  ├──────────────────>│
    │                  │                   │
    │                  │  job runs         │
    │                  │<──────────────────│
    │                  │                   │
    │   "Done!"        │                   │
    │<─────────────────│                   │
```

## Key Files to Know

| File | What It Does |
|------|--------------|
| `README.md` | Complete architecture and documentation |
| `SETUP.md` | Detailed step-by-step setup instructions |
| `PROJECT_STRUCTURE.md` | Quick reference for all files |
| `quickstart.sh` | Interactive setup script |
| `Makefile` | Common commands (make help) |

## Common Tasks

### View System Status
```bash
make status
```

### Start the Orchestrator (on M3)
```bash
make run-orchestrator
```

### Deploy to Cluster (on Master Pi)
```bash
make deploy-cluster
```

### Use the Client (on Laptop)
```bash
make run-client
```

### View Logs
```bash
make logs-orchestrator  # M3 logs
make logs-cluster       # Cluster logs
```

## The "Crawl, Walk, Run" Phases

We built this system in three phases. You can stop at any phase:

### Phase 1: Crawl (Local Testing)
Just test the Master Agent locally on your M3.
**Goal**: Verify Ollama + CrewAI + Tools work

### Phase 2: Walk (API Server)
Run the Master Agent as an API server.
**Goal**: Allow remote access from your laptop

### Phase 3: Run (Full System)
Connect everything together.
**Goal**: Client → M3 → Cluster all working

Most people will want Phase 3 (full system).

## What You Can Do Now

Try these example tasks with your client:

```python
# Check cluster health
"How is my cluster doing?"

# List resources
"Show me all pods in the ai-services namespace"

# Deploy something
"Deploy the nginx test app to the cluster"

# Spawn a worker agent
"Review the code in my repository"
```

## Troubleshooting

**Can't connect to M3 from laptop:**
```bash
# Check network
ping <m3-ip>

# Check if API is running
curl http://<m3-ip>:8000/health
```

**Master Agent can't reach cluster:**
```bash
# On M3, test kubectl
kubectl get nodes

# Check kubeconfig
cat ~/.kube/config
```

**Ollama not working:**
```bash
# Check if running
curl http://localhost:11434/api/version

# Start it
ollama serve

# Pull a model
ollama pull llama3.1:8b
```

## Next Steps

Once the basic system is working:

1. **Add Authentication** - Secure the API with API keys
2. **Create Custom Tools** - Add your own tools to the Master Agent
3. **Build Worker Agents** - Create specialized agents for your needs
4. **Deploy Your Apps** - Move your applications to the cluster
5. **Add Monitoring** - Set up Prometheus/Grafana

## Need Help?

- **Quick reference**: `PROJECT_STRUCTURE.md`
- **Detailed setup**: `SETUP.md`
- **Architecture**: `README.md`
- **Common commands**: `make help`

## Philosophy

Remember the key concept:

> **The M3 is the CEO. The Pi Cluster is the factory floor.**
> 
> **Don't put the CEO on the factory floor.**

The M3 does the heavy thinking with your largest model. The cluster executes the work. They're connected but separate.

This gives you:
- **Power** - Use the biggest models on M3 for planning
- **Scale** - Distribute work across the cluster
- **Flexibility** - Add/remove cluster nodes without affecting the brain
- **Clarity** - Clean separation of concerns

---

**Ready to start?** Run `./quickstart.sh` and follow the prompts!
