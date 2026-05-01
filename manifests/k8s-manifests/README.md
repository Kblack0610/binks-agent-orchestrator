# Kubernetes manifests

Cluster manifests for shared infrastructure used by the MCP servers.

```
manifests/k8s-manifests/
├── core/
│   ├── namespace.yaml          # ai-services namespace
│   └── searxng-deployment.yaml # SearXNG backend for web-search-mcp
└── README.md
```

## Apply

```bash
kubectl apply -f core/namespace.yaml
kubectl apply -f core/searxng-deployment.yaml
```

## History

The Binks Agent's k8s manifests (`binks-agent-deployment.yaml`, `binks-agent-ingress.yaml`, `ollama-deployment.yaml`, `ollama-nodeport.yaml`, `agents/code-reviewer-job.yaml`) were extracted to `~/dev/home/binks/manifests/` (archived) on 2026-04-30 along with the agent itself.
