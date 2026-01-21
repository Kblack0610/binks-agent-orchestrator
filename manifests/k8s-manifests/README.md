# Kubernetes Manifests

Deployment manifests for the Binks Agent infrastructure on DigitalOcean Kubernetes.

## Architecture

```
binks.chat (DNS) → DO Load Balancer (209.38.61.219)
                         ↓
                  nginx ingress (TLS via Let's Encrypt)
                         ↓
              binks-agent (ai-services namespace)
                    ↓         ↓
            SearXNG      Ollama (external: 192.168.1.4)
```

## Namespaces

- `ai-services` - Core infrastructure (agent, SearXNG, etc.)
- `ai-agents` - Worker agent jobs (code reviewer, etc.)

## Prerequisites

1. **DNS Configuration**: Point `binks.chat` to the DO load balancer IP (`209.38.61.219`)
2. **cert-manager**: ClusterIssuer `letsencrypt` must exist
3. **nginx-ingress**: Ingress controller must be running
4. **Storage**: `do-block-storage` StorageClass for PVCs

## Deployment Steps

### 1. Build and Push Docker Image

The GitHub Actions workflow (`.github/workflows/docker.yml`) automatically builds and pushes on:
- Push to `master` branch (tagged as `latest`)
- Git tags (tagged with version)

**Manual build:**

```bash
# From repo root
docker build -f Dockerfile.agent -t ghcr.io/kblack0610/binks-agent:latest .
docker push ghcr.io/kblack0610/binks-agent:latest
```

### 2. Create Namespace (if not exists)

```bash
kubectl apply -f core/namespace.yaml
```

### 3. Deploy SearXNG (search backend)

```bash
kubectl apply -f core/searxng-deployment.yaml
```

### 4. Deploy Binks Agent

```bash
kubectl apply -f core/binks-agent-deployment.yaml
kubectl apply -f core/binks-agent-ingress.yaml
```

### 5. Verify Deployment

```bash
# Check pods
kubectl get pods -n ai-services

# Check ingress and TLS certificate
kubectl get ingress -n ai-services
kubectl get certificate -n ai-services

# Test health endpoint
curl https://binks.chat/api/health
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OLLAMA_URL` | `http://192.168.1.4:11434` | Ollama API endpoint |
| `OLLAMA_MODEL` | `llama3.1:8b` | Default model to use |
| `BINKS_DATA_DIR` | `/data` | SQLite database location |
| `RUST_LOG` | `info,agent=debug` | Log level |

### Resource Limits

The agent is configured for small DO nodes (s-1vcpu-2gb):

- Requests: 64Mi memory, 25m CPU
- Limits: 256Mi memory, 500m CPU

### Storage

- 1Gi PVC for SQLite database
- Uses `do-block-storage` StorageClass

## Troubleshooting

### Pod not starting

```bash
kubectl describe pod -n ai-services -l app=binks-agent
kubectl logs -n ai-services -l app=binks-agent
```

### TLS certificate not issued

```bash
kubectl describe certificate binks-chat-tls -n ai-services
kubectl describe clusterissuer letsencrypt
```

### WebSocket connection issues

The ingress is configured with WebSocket support. If connections drop:

```bash
# Check ingress annotations
kubectl get ingress binks-agent -n ai-services -o yaml
```

## Files

| File | Purpose |
|------|---------|
| `core/namespace.yaml` | Namespace definitions |
| `core/searxng-deployment.yaml` | SearXNG search backend |
| `core/binks-agent-deployment.yaml` | Agent Deployment + Service + PVC |
| `core/binks-agent-ingress.yaml` | Ingress with TLS for binks.chat |
| `agents/code-reviewer-job.yaml` | Example agent job |
