# Global AI - Setup Guide

This guide will walk you through setting up your Global AI system from scratch.

## Overview

You'll be setting up three components across three different machines:

1. **Cluster** (Raspberry Pi cluster) - The execution plane
2. **Orchestrator** (M3 Ultra) - The AI control plane
3. **Client** (Your laptop) - The interface

## Prerequisites Checklist

- [ ] Kubernetes cluster already running on your Pi cluster
- [ ] M3 Ultra with macOS
- [ ] Client machine (laptop/desktop)
- [ ] All machines can reach each other on the network
- [ ] Git installed on all machines
- [ ] Python 3.11+ installed on M3 and client

## Step-by-Step Setup

### Part 1: Initial Repository Setup

On your **development machine** (where you're working now):

```bash
# You're already here, just initialize git if needed
cd /home/kblack0610/dev/global

# Initialize git repository
git init

# Add all files
git add .

# Initial commit
git commit -m "Initial commit: Global AI infrastructure setup"

# Add remote (GitHub, GitLab, or your own git server)
git remote add origin <your-git-repo-url>

# Push
git push -u origin main
```

### Part 2: M3 Ultra Setup (Orchestrator)

SSH into your M3 Ultra:

```bash
ssh user@m3-ultra.local  # Replace with your M3's hostname/IP
```

Then run:

```bash
# 1. Clone the repository
cd ~
git clone <your-git-repo-url> global
cd global/orchestrator

# 2. Install Homebrew (if not already installed)
# /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# 3. Install Ollama
brew install ollama

# 4. Start Ollama service (in a separate terminal or use tmux/screen)
ollama serve

# 5. Pull your model (this will take a while for 405B!)
# In another terminal:
ollama pull llama3.1:405b
# Or use a smaller model for testing first:
ollama pull llama3.1:8b

# 6. Set up Python environment
python3 -m venv venv
source venv/bin/activate

# 7. Install dependencies
pip install -r requirements/base.txt

# 8. Configure environment
cp .env.example .env
nano .env  # Edit with your settings

# 9. Set up kubectl access to your cluster
# On your master pi, run: cat ~/.kube/config
# Copy the output
mkdir -p ~/.kube
nano ~/.kube/config  # Paste the kubeconfig

# 10. Test kubectl connection
kubectl get nodes
kubectl cluster-info

# 11. Test the orchestrator locally (Crawl phase)
python src/main.py

# Try asking: "What is the status of my cluster?"

# 12. If local test works, start the API server (Walk phase)
# Press Ctrl+C to exit the local test, then:
python src/api/server.py

# The API will be running at http://0.0.0.0:8000
# Leave this running (or set up as systemd service - see below)
```

#### Optional: Set up as systemd service (recommended)

```bash
# Create service file
sudo nano /etc/systemd/system/orchestrator.service
```

Paste this content (adjust paths for your user):

```ini
[Unit]
Description=Binks Orchestrator AI Control Plane
After=network.target

[Service]
Type=simple
User=your-username
WorkingDirectory=/Users/your-username/global/orchestrator
Environment="PATH=/Users/your-username/global/orchestrator/venv/bin"
ExecStart=/Users/your-username/global/orchestrator/venv/bin/python src/api/server.py
Restart=always

[Install]
WantedBy=multi-user.target
```

Then:

```bash
sudo systemctl daemon-reload
sudo systemctl enable orchestrator
sudo systemctl start orchestrator
sudo systemctl status orchestrator
```

### Part 3: Cluster Setup (Raspberry Pis)

SSH into your **master pi**:

```bash
ssh user@master-pi.local
```

Then run:

```bash
# 1. Clone the repository
cd ~
git clone <your-git-repo-url> global
cd global/cluster

# 2. Create namespaces
kubectl apply -f k8s-manifests/core/namespace.yaml

# Verify
kubectl get namespaces

# 3. Deploy Ollama service (for worker agents)
kubectl apply -f k8s-manifests/core/ollama-deployment.yaml

# Wait for it to be ready
kubectl get pods -n ai-services -w

# 4. Once the pod is running, pull a lightweight model
kubectl exec -n ai-services deployment/ollama -- ollama pull llama3.1:8b

# Verify
kubectl logs -n ai-services deployment/ollama
```

### Part 4: Client Setup (Your Laptop)

On your **laptop/client machine**:

```bash
# 1. Clone the repository
cd ~
git clone <your-git-repo-url> global
cd global/client

# 2. Install Python dependencies
pip install requests pyyaml

# 3. Configure API endpoint
nano config/api-endpoints.yaml

# Update the 'local' environment with your M3's IP address:
# host: "192.168.1.XXX"  # Your M3's IP

# 4. Test connection
python src/simple_client.py --health

# Expected output:
# Orchestrator Health:
#   status: healthy
#   agent: ready
#   ...

# 5. Test cluster status
python src/simple_client.py --cluster

# 6. Try interactive mode
python src/simple_client.py

# Try asking: "List all pods in the cluster"
```

## Verification Tests

Run these tests to verify everything is working:

### Test 1: M3 can reach cluster

On your M3:
```bash
kubectl get nodes
kubectl get pods --all-namespaces
```

Should show your cluster nodes and pods.

### Test 2: Client can reach M3

On your client:
```bash
curl http://<m3-ip>:8000/health
```

Should return JSON with status "healthy".

### Test 3: End-to-end test

On your client:
```bash
python src/simple_client.py
```

Then ask: "What pods are running in the ai-services namespace?"

The Master Agent should:
1. Receive your request
2. Use its `run_kubectl` tool
3. Return the list of pods

### Test 4: Spawn a worker agent

On your client:
```bash
python src/simple_client.py
```

Then ask: "Spawn a code-reviewer agent to check the placemyparents repository"

The Master Agent should:
1. Use the `spawn_worker_agent` tool
2. Create a Kubernetes Job on your cluster
3. Return the job name

Verify on master pi:
```bash
kubectl get jobs -n ai-agents
kubectl logs job/<job-name> -n ai-agents
```

## Troubleshooting

### M3 can't reach cluster

**Problem**: `kubectl` commands fail with connection errors

**Solution**:
```bash
# Check kubeconfig
cat ~/.kube/config

# Test network to master pi
ping <master-pi-ip>

# Verify the API server address in kubeconfig matches your master pi
```

### Client can't reach M3

**Problem**: `curl http://<m3-ip>:8000/health` fails

**Solution**:
```bash
# On M3, check if service is running
curl http://localhost:8000/health  # Should work locally

# Check firewall
sudo ufw status
sudo ufw allow 8000/tcp

# Check if listening on all interfaces
netstat -an | grep 8000
# Should show 0.0.0.0:8000 not 127.0.0.1:8000
```

### Ollama not working

**Problem**: "Connection refused" to Ollama

**Solution**:
```bash
# Check if Ollama is running
curl http://localhost:11434/api/version

# If not, start it
ollama serve

# Check available models
ollama list

# Pull model if missing
ollama pull llama3.1:8b  # Start with smaller model for testing
```

### Worker agents fail to spawn

**Problem**: `spawn_worker_agent` returns errors

**Solution**:
```bash
# Check if namespaces exist
kubectl get namespace ai-agents

# Check if job template exists
ls -la cluster/k8s-manifests/agents/

# Check Ollama service is running in cluster
kubectl get pods -n ai-services

# Check job logs
kubectl get jobs -n ai-agents
kubectl describe job <job-name> -n ai-agents
kubectl logs job/<job-name> -n ai-agents
```

## Network Configuration Quick Reference

| Component | Machine | IP Example | Port | Notes |
|-----------|---------|------------|------|-------|
| Master Pi | Pi Cluster | 192.168.1.100 | 6443 | K8s API server |
| M3 Orchestrator | M3 Ultra | 192.168.1.101 | 8000 | FastAPI server |
| Ollama (M3) | M3 Ultra | localhost | 11434 | Local only |
| Ollama (Cluster) | Pi Cluster | ClusterIP | 11434 | Internal only |
| Client | Laptop | 192.168.1.102 | - | Accesses M3:8000 |

## Next Steps

Once everything is working:

1. [ ] Test with a real task (e.g., "Deploy placemyparents")
2. [ ] Create your first custom worker agent
3. [ ] Add authentication to the FastAPI server
4. [ ] Set up monitoring and logging
5. [ ] Build custom workflows for your needs

## Getting Help

If you run into issues:

1. Check the logs:
   - M3: `tail -f ~/global/orchestrator/logs/orchestrator.log`
   - Cluster: `kubectl logs <pod-name> -n <namespace>`

2. Run health checks:
   - M3: `curl http://localhost:8000/health`
   - Cluster: `kubectl get pods --all-namespaces`

3. Verify network connectivity:
   - `ping <machine-ip>`
   - `curl http://<machine-ip>:<port>/health`

## Congratulations!

If all tests pass, you now have a working Global AI system!

Your M3 Ultra is the "Brain", your Pi Cluster is the "Body", and you have a clean interface to control it all.
