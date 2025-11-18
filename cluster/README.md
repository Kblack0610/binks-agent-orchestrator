# Cluster - Compute Plane

This directory contains the Kubernetes cluster configuration for your Pi Cluster (the "Compute Plane").

## Architecture

Your cluster runs:
- **Kubernetes Control Plane** (on your "master pi")
- **Worker Nodes** (on your other Pis and CPUs)
- **Applications** (like `placemyparents`) as pods
- **Worker Agents** (spawned as Kubernetes Jobs by the M3 Orchestrator)

## Directory Structure

```
cluster/
├── k8s-manifests/
│   ├── core/           # Core cluster services (Ollama, monitoring, etc.)
│   ├── apps/           # Your applications (placemyparents, etc.)
│   └── agents/         # Agent job templates for spawning worker agents
├── docs/               # Cluster documentation
├── scripts/            # Deployment and management scripts
└── README.md          # This file
```

## Prerequisites

- Kubernetes cluster already set up on your Pi Cluster
- `kubectl` configured to access your cluster
- Network connectivity between all nodes

## Quick Start

### 1. Verify Cluster Access

From your master pi (or any machine with kubectl configured):

```bash
kubectl cluster-info
kubectl get nodes
```

### 2. Deploy Core Services

```bash
# Deploy Ollama service (for worker agents to use smaller models)
kubectl apply -f k8s-manifests/core/ollama-deployment.yaml

# Verify
kubectl get pods -n ai-services
```

### 3. Deploy Your Applications

```bash
# Example: Deploy placemyparents
kubectl apply -f k8s-manifests/apps/placemyparents/

# Verify
kubectl get pods -l app=placemyparents
```

## Connecting to the M3 Orchestrator

The M3 Ultra (binks-orchestrator) does **not** run as part of this cluster.
It connects to this cluster as a **client** using kubectl.

To allow the M3 to manage this cluster:

1. **On your master pi**, copy the kubeconfig:
   ```bash
   cat ~/.kube/config
   ```

2. **On your M3 Ultra**, save it to `~/.kube/config`

3. **Test the connection** from your M3:
   ```bash
   kubectl get nodes
   ```

Now the CrewAI Master Agent on your M3 can use its `run_kubectl` tool to manage this cluster.

## Cluster Services

### Ollama Service (for Worker Agents)

Worker agents spawned as Kubernetes Jobs will use a **lightweight Ollama instance** running on the cluster (e.g., Llama 3 8B) for quick tasks.

- **Service Name**: `ollama-service.ai-services.svc.cluster.local`
- **Port**: `11434`
- **Model**: `llama3:8b` (or similar small model)

### Monitoring

(To be added: Prometheus, Grafana for cluster metrics)

## Agent Job Templates

Worker agents are spawned using Kubernetes Job manifests in `k8s-manifests/agents/`.

Example: When the M3 Master Agent needs a "CodeReviewer" agent, it runs:

```bash
kubectl apply -f k8s-manifests/agents/code-reviewer-job.yaml
```

This creates a temporary pod that:
1. Pulls the latest code
2. Runs analysis using the cluster's Ollama service
3. Reports results back
4. Terminates

## Deployment Workflow

```
[You code locally]
    ↓
[Commit to git]
    ↓
[M3 Orchestrator detects change or receives command]
    ↓
[M3 spawns "Deployer" agent via kubectl apply]
    ↓
[Job runs on Pi Cluster]
    ↓
[Your app is deployed/updated]
```

## Network Configuration

- **Master Pi IP**: (Add your master pi IP here, e.g., 192.168.1.100)
- **Cluster API Endpoint**: `https://<master-pi-ip>:6443`
- **Service CIDR**: (Your cluster's service network)
- **Pod CIDR**: (Your cluster's pod network)

## Troubleshooting

### Can't connect from M3 to cluster
```bash
# On M3, test network connectivity
ping <master-pi-ip>

# Test kubectl
kubectl cluster-info

# Check kubeconfig
cat ~/.kube/config
```

### Pods stuck in Pending
```bash
kubectl describe pod <pod-name>
kubectl get events --sort-by=.metadata.creationTimestamp
```

## Next Steps

1. [ ] Deploy Ollama service to the cluster
2. [ ] Set up kubeconfig on M3 Ultra
3. [ ] Create first agent job template
4. [ ] Test spawning an agent from M3
