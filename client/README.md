# Binks Client

Python client library for communicating with the Binks Orchestrator API.

## Overview

```
┌──────────────────────────────────────────────┐
│         Your Machine (Client)                 │
│                                              │
│  ┌────────────────────────────────────────┐ │
│  │         simple_client.py               │ │
│  │                                        │ │
│  │  $ python simple_client.py             │ │
│  │  You: Check cluster status             │ │
│  │                                        │ │
│  └────────────┬───────────────────────────┘ │
│               │                              │
└───────────────┼──────────────────────────────┘
                │
                │ HTTP Request
                │
┌───────────────▼──────────────────────────────┐
│         M3 Ultra (Binks Orchestrator)        │
│                                              │
│  ┌────────────────────────────────────────┐ │
│  │       FastAPI Server (Port 8000)       │ │
│  │              ▼                         │ │
│  │       Agno Master Agent                │ │
│  │              ▼                         │ │
│  │       Ollama (405B Model)              │ │
│  └────────────────────────────────────────┘ │
│                                              │
└──────────────────────────────────────────────┘
```

## Directory Structure

```
client/
├── src/
│   └── simple_client.py   # Python CLI client
├── config/
│   └── api-endpoints.yaml # API endpoint configuration
└── README.md              # This file
```

## Prerequisites

1. **Python 3.8+** with `requests` and `pyyaml`
2. **Network access** to your M3 Ultra
3. **Binks Orchestrator** running on M3

## Installation

```bash
cd client
pip install requests pyyaml
```

## Configuration

Edit `config/api-endpoints.yaml` with your M3's IP address:

```yaml
default: local

environments:
  local:
    host: "192.168.1.XXX"  # Replace with your M3 IP
    port: 8000
    protocol: "http"
```

## Usage

### Interactive Mode

```bash
python src/simple_client.py
```

```
============================================================
Binks Client - Interactive Mode
Connected to: http://192.168.1.100:8000
============================================================

Type your tasks or 'quit' to exit.

You: Check cluster status

Thinking...

Agent:
All 4 nodes are Ready. 15 pods running across namespaces.
```

### Single Command Mode

```bash
python src/simple_client.py "Get all pods in the default namespace"
```

### Health Check

```bash
python src/simple_client.py --health
```

### Cluster Status

```bash
python src/simple_client.py --cluster
```

### Different Environment

```bash
python src/simple_client.py --env production "Deploy latest version"
```

## Direct API Calls

You can also call the API directly with curl:

```bash
# Health check
curl http://<m3-ip>:8000/health

# Invoke agent
curl -X POST http://<m3-ip>:8000/invoke \
  -H "Content-Type: application/json" \
  -d '{"task": "Get the status of all pods"}'

# With context
curl -X POST http://<m3-ip>:8000/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "task": "Review code changes",
    "context": {
      "repo": "placemyparents",
      "branch": "main"
    }
  }'
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/invoke` | POST | Send task to agent |
| `/cluster/status` | POST | Get cluster status |
| `/agent/info` | GET | Get agent information |

## Troubleshooting

### Can't connect to orchestrator

```bash
# 1. Check network connectivity
ping <m3-ip>

# 2. Check if orchestrator is running
curl http://<m3-ip>:8000/health

# 3. Check firewall on M3
sudo ufw allow 8000/tcp
```

### Slow responses

Normal for 405B model. For faster dev responses:
1. Use smaller model (8B) on M3
2. Run agent locally: `python orchestrator/agno/src/agent.py`
