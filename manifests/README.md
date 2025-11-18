# Kubernetes Manifests

This directory contains **Kubernetes deployment manifests** for the AI services that run on your **existing cluster**.

## Important: This is NOT Cluster Setup

❌ This is **NOT** how to install/configure Kubernetes
❌ This is **NOT** cluster infrastructure setup
✅ This **IS** application deployments for your existing cluster
✅ These deploy **TO** your cluster **FROM** your laptop (where you run kubectl)

## What's In Here

Application manifests that deploy AI services to your cluster:

```
manifests/
├── k8s-manifests/
│   ├── core/
│   │   ├── namespace.yaml              # Creates ai-services & ai-agents namespaces
│   │   └── ollama-deployment.yaml      # Ollama service for worker agents
│   ├── apps/
│   │   └── (your applications go here)
│   └── agents/
│       └── code-reviewer-job.yaml      # Template for spawned worker agents
└── scripts/
```

## How You Use This

### You Already Have:
- ✅ A running Kubernetes cluster (Pi cluster + other nodes)
- ✅ kubectl configured on your laptop
- ✅ Other apps running on the cluster (placemyparents, etc.)

### Deploy From Your Laptop:

```bash
# You're on your laptop (where you manage kubectl)
cd ~/global/manifests

# Deploy the AI services to your existing cluster
kubectl apply -f k8s-manifests/core/namespace.yaml
kubectl apply -f k8s-manifests/core/ollama-deployment.yaml

# Verify (you'll see your existing apps + new AI services)
kubectl get pods --all-namespaces
```

### What Gets Deployed:

1. **Namespaces** (`ai-services`, `ai-agents`)
   - Organizes AI-related workloads
   - Keeps them separate from your other apps

2. **Ollama Service** (in `ai-services` namespace)
   - Runs a lightweight Ollama instance (e.g., Llama 3.1 8B)
   - Used by worker agents for quick tasks
   - Separate from the big Ollama on your M3

3. **Worker Agent Templates** (in `agents/`)
   - Job manifests used by the orchestrator
   - The M3 orchestrator spawns these as needed
   - They run, do their task, and terminate

## Architecture

```
Your Laptop
    ↓
  kubectl apply
    ↓
Your Existing Kubernetes Cluster
├── Your existing apps (placemyparents, etc.)
├── [NEW] Ollama service (ai-services namespace)
└── [NEW] Worker agent jobs (ai-agents namespace, spawned by M3)
```

## Deployment Workflow

### Initial Setup (One Time):

```bash
# From your laptop (where you already manage kubectl)
cd ~/global/manifests

# Deploy AI services
kubectl apply -f k8s-manifests/core/

# Verify namespaces created
kubectl get namespaces | grep ai-

# Verify Ollama is running
kubectl get pods -n ai-services

# Pull a model into the cluster's Ollama
kubectl exec -n ai-services deployment/ollama -- ollama pull llama3.1:8b
```

### Normal Operation:

Once deployed, the M3 orchestrator will:
1. Spawn worker agents using the templates in `k8s-manifests/agents/`
2. These run as Kubernetes Jobs on your cluster
3. They use the Ollama service you deployed
4. They report results back to the M3 and terminate

You don't deploy the worker agents manually - the orchestrator does it.

## What Runs Where

| Component | Where It Runs | How You Deploy It |
|-----------|---------------|-------------------|
| **Orchestrator** | M3 Ultra | Run Python directly on M3 |
| **Client** | Your Laptop | Run Python on your laptop |
| **Ollama Service** | Your Cluster | `kubectl apply` from laptop |
| **Worker Agents** | Your Cluster | Spawned by orchestrator via kubectl |
| **Your Existing Apps** | Your Cluster | (Already deployed your way) |

## Connecting the M3 Orchestrator

The M3 Ultra orchestrator needs kubectl access to spawn worker agents.

Copy your kubeconfig from your laptop to the M3:

```bash
# On your laptop, display your kubeconfig
cat ~/.kube/config

# On your M3 Ultra, save it
mkdir -p ~/.kube
nano ~/.kube/config  # Paste the config

# Test from M3
kubectl get nodes
```

Now the M3 can spawn worker agents on your cluster.

## Adding Your Applications

Put your app manifests in `k8s-manifests/apps/`:

```bash
manifests/k8s-manifests/apps/
├── placemyparents/
│   ├── deployment.yaml
│   ├── service.yaml
│   └── ingress.yaml
└── your-other-app/
    └── deployment.yaml
```

Then deploy from your laptop:
```bash
kubectl apply -f k8s-manifests/apps/placemyparents/
```

## Customizing Ollama Deployment

Edit `k8s-manifests/core/ollama-deployment.yaml`:

```yaml
# Change model size based on cluster resources
# Default: Uses small model (8B) suitable for Pi cluster

# For more powerful cluster:
spec:
  containers:
  - name: ollama
    resources:
      limits:
        memory: "16Gi"  # Increase for larger models
        cpu: "4"
```

## Network Configuration

The Ollama service is accessible:
- **Within cluster**: `ollama-service.ai-services.svc.cluster.local:11434`
- **From outside**: Not exposed (ClusterIP only)

## Monitoring

Check status of AI services:

```bash
# All AI-related pods
kubectl get pods -n ai-services
kubectl get pods -n ai-agents

# Ollama logs
kubectl logs -n ai-services deployment/ollama

# Worker agent jobs
kubectl get jobs -n ai-agents
kubectl logs job/<job-name> -n ai-agents
```

## Cleanup

To remove AI services from your cluster:

```bash
# Remove all AI services (keeps your other apps)
kubectl delete namespace ai-services
kubectl delete namespace ai-agents
```

Your other applications are unaffected.

## Troubleshooting

### Ollama pod won't start

```bash
# Check events
kubectl describe pod -n ai-services -l app=ollama

# Common issues:
# - Not enough memory (needs ~2Gi minimum)
# - Image pull issues (check internet connectivity)
```

### Worker agents failing

```bash
# Check job status
kubectl get jobs -n ai-agents

# View logs
kubectl logs job/<job-name> -n ai-agents

# Check if Ollama service is reachable
kubectl exec -n ai-agents <pod-name> -- curl ollama-service.ai-services:11434/api/version
```

## Next Steps

1. [ ] Deploy these manifests from your laptop using kubectl
2. [ ] Set up kubectl access on your M3 Ultra
3. [ ] Test spawning a worker agent from the orchestrator

See the main README.md for complete system setup instructions.
