# Binks Orchestrator API Guide (Walk Phase)

This guide covers the Agno-based API implementation for the Binks Orchestrator.

## Overview

The Binks Orchestrator API provides a lightweight, high-performance interface to the Master Agent using the Agno framework. This is the "Walk" phase of development, where the agent is exposed via a REST API.

## Quick Start

### 1. Install Dependencies

```bash
cd orchestrator/agno
python -m pip install -r requirements.txt
```

### 2. Configure Environment

The API uses the `.env` file for configuration:

```bash
# Ollama Configuration
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=llama3.1:8b

# API Configuration
AGNO_API_HOST=0.0.0.0
AGNO_API_PORT=8000
```

### 3. Start the Server

```bash
cd src
python -m api.server
```

The server will start on `http://0.0.0.0:8000` by default.

## API Endpoints

### Health Check

**GET /** - Basic health check

```bash
curl http://localhost:8000/
```

Response:
```json
{
  "status": "running",
  "service": "Binks Orchestrator (Agno)",
  "version": "1.0.0-walk",
  "phase": "walk"
}
```

### Detailed Health Check

**GET /health** - Detailed health information

```bash
curl http://localhost:8000/health
```

Response:
```json
{
  "status": "healthy",
  "agent": "ready",
  "agent_name": "MasterOrchestrator",
  "ollama_url": "http://localhost:11434",
  "ollama_model": "llama3.1:8b",
  "implementation": "agno"
}
```

### Agent Information

**GET /agent/info** - Get agent details

```bash
curl http://localhost:8000/agent/info
```

Response:
```json
{
  "name": "MasterOrchestrator",
  "model": "llama3.1:8b",
  "tools": ["KubectlToolkit", "AgentSpawnerToolkit"],
  "implementation": "agno",
  "phase": "walk"
}
```

### Invoke Agent

**POST /invoke** - Send a task to the agent

```bash
curl -X POST http://localhost:8000/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "task": "What is the status of the cluster?",
    "context": {
      "priority": "high"
    }
  }'
```

Request Body:
```json
{
  "task": "string (required)",
  "context": {
    "key": "value (optional)"
  }
}
```

Response:
```json
{
  "success": true,
  "result": "Agent response here...",
  "error": null
}
```

### Cluster Status

**POST /cluster/status** - Get cluster status directly

```bash
curl -X POST http://localhost:8000/cluster/status
```

Response:
```json
{
  "status": "cluster status information"
}
```

## Interactive Documentation

FastAPI automatically generates interactive API documentation:

- **Swagger UI**: http://localhost:8000/docs
- **ReDoc**: http://localhost:8000/redoc

## Example Usage

### Python

```python
import requests

# Invoke the agent
response = requests.post(
    "http://localhost:8000/invoke",
    json={
        "task": "List all pods in the default namespace",
        "context": {"namespace": "default"}
    }
)

result = response.json()
print(result["result"])
```

### cURL

```bash
curl -X POST http://localhost:8000/invoke \
  -H "Content-Type: application/json" \
  -d '{"task": "Get cluster status"}'
```

## Available Tools

The Master Agent has access to the following tools:

1. **run_kubectl**: Execute kubectl commands on the cluster
2. **get_cluster_status**: Quick health check of the cluster
3. **spawn_worker_agent**: Create a Kubernetes Job for a specialized agent
4. **check_agent_status**: Monitor spawned worker agents

## Error Handling

The API returns standard HTTP status codes:

- **200**: Success
- **503**: Service unavailable (agent not initialized)
- **500**: Internal server error

Error responses include details:
```json
{
  "success": false,
  "result": "",
  "error": "Error message here"
}
```

## Development Mode

The server runs in development mode with auto-reload enabled. Any changes to the code will automatically restart the server.

To disable auto-reload for production:

Edit `src/api/server.py`:
```python
uvicorn.run(
    "api.server:app",
    host=host,
    port=port,
    reload=False  # Set to False for production
)
```

## Next Steps (Run Phase)

The "Run" phase will include:
- Dockerization
- Kubernetes deployment manifests
- Horizontal scaling
- Monitoring and observability
- Production-ready configuration

## Troubleshooting

### Agent not initializing

Check that Ollama is running:
```bash
ollama list
ps aux | grep ollama
```

### Port already in use

Change the port in `.env`:
```bash
AGNO_API_PORT=8001
```

### Module not found errors

Ensure all dependencies are installed:
```bash
python -m pip install -r requirements.txt
```
