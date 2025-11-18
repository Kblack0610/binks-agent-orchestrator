# Binks Client - OpenCode TUI Interface

This is your **client interface** - the "window" into your Global AI system using the `opencode` TUI.

## What This Is

The Binks Client is a lightweight wrapper that configures `opencode` to talk to your **Binks Orchestrator** (running on the M3 Ultra).

```
┌──────────────────────────────────────────────┐
│         Your Laptop (Binks Client)           │
│                                              │
│  ┌────────────────────────────────────────┐ │
│  │         opencode TUI                   │ │
│  │                                        │ │
│  │  "Deploy placemyparents v2.0"          │ │
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
│  │       CrewAI Master Agent              │ │
│  │              ▼                         │ │
│  │       Ollama (405B Model)              │ │
│  └────────────────────────────────────────┘ │
│                                              │
└──────────────────────────────────────────────┘
```

## Directory Structure

```
binks-client/
├── src/
│   └── opencode-config/    # opencode configuration files
├── config/
│   └── api-endpoints.yaml  # API endpoint configuration
├── scripts/
│   └── start-opencode.sh   # Script to launch opencode
└── README.md              # This file
```

## Prerequisites

1. **opencode installed** on your laptop/client machine
2. **Network access** to your M3 Ultra
3. **Binks Orchestrator** running on M3 (see ../binks-orchestrator/README.md)

## Installation

### 1. Install opencode

Follow the instructions at: https://github.com/anthropics/opencode

```bash
# Example (check official docs for latest)
pip install opencode
```

### 2. Configure API Endpoint

Edit `config/api-endpoints.yaml` with your M3's IP address:

```yaml
orchestrator:
  host: "192.168.1.XXX"  # Replace with your M3 IP
  port: 8000
  protocol: "http"
```

### 3. Test Connection

```bash
# Test that the orchestrator is reachable
curl http://<m3-ip>:8000/health
```

You should see:
```json
{
  "status": "healthy",
  "agent": "ready",
  "ollama_url": "http://localhost:11434",
  "ollama_model": "llama3.1:405b"
}
```

## Usage

### Option 1: Direct API Calls (for testing)

Before using opencode, test the API directly:

```bash
# Simple task
curl -X POST http://<m3-ip>:8000/invoke \
  -H "Content-Type: application/json" \
  -d '{"task": "Get the status of all pods in the cluster"}'

# Task with context
curl -X POST http://<m3-ip>:8000/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "task": "Review the latest code changes",
    "context": {
      "repo_url": "https://github.com/user/repo",
      "branch": "main"
    }
  }'
```

### Option 2: Using opencode TUI

Launch opencode configured to use your orchestrator:

```bash
cd binks-client
./scripts/start-opencode.sh
```

This will:
1. Read the API endpoint from `config/api-endpoints.yaml`
2. Launch opencode with the correct configuration
3. Present you with the TUI interface

Then you can type natural language commands like:
- "Show me all running pods"
- "Deploy the latest version of placemyparents"
- "Check the health of the cluster"
- "Review the code in the main branch and tell me if there are any issues"

### Option 3: Custom Client Script

You can also build your own simple client:

```python
# src/simple_client.py
import requests

ORCHESTRATOR_URL = "http://192.168.1.XXX:8000"

def ask_agent(task: str, context: dict = None):
    """Send a task to the Master Agent."""
    payload = {"task": task}
    if context:
        payload["context"] = context

    response = requests.post(
        f"{ORCHESTRATOR_URL}/invoke",
        json=payload
    )

    result = response.json()
    if result["success"]:
        print("Agent Response:")
        print(result["result"])
    else:
        print("Error:")
        print(result.get("error", "Unknown error"))

# Use it
ask_agent("What pods are running on the cluster?")
```

## Example Workflows

### 1. Deploy an Application

```
You: Deploy the latest version of placemyparents to the cluster

Agent:
- Checking current deployment status...
- Pulling latest image...
- Updating deployment manifest...
- Spawning deployment-verifier agent...
- Verifying rollout...
- ✓ Deployment complete! 3/3 pods running.
```

### 2. Code Review

```
You: Review the latest changes in the placemyparents repo and tell me if there are any issues

Agent:
- Spawning code-reviewer agent...
- Cloning repository...
- Analyzing changes...
- Running linters...
- Code review complete. Found 2 suggestions:
  1. Line 42: Consider adding error handling
  2. Line 78: Potential memory leak
```

### 3. Cluster Health Check

```
You: How is the cluster doing?

Agent:
- Checking cluster status...
- All 4 nodes are Ready
- 15 pods running (14 running, 1 pending)
- Recent events:
  - placemyparents-web-1 restarted 5 minutes ago
- Overall status: Healthy ✓
```

## Configuration

### Connecting to Different Orchestrators

If you have multiple orchestrators (e.g., dev, staging, production):

```yaml
# config/api-endpoints.yaml
environments:
  dev:
    host: "192.168.1.100"
    port: 8000
  staging:
    host: "192.168.1.101"
    port: 8000
  production:
    host: "10.0.0.50"
    port: 8000

default: dev
```

Then use:
```bash
./scripts/start-opencode.sh --env production
```

## Troubleshooting

### Can't connect to orchestrator

```bash
# 1. Check network connectivity
ping <m3-ip>

# 2. Check if orchestrator is running
curl http://<m3-ip>:8000/health

# 3. Check firewall on M3
# On M3:
sudo ufw status
sudo ufw allow 8000/tcp  # If needed
```

### opencode not working

```bash
# Check opencode installation
opencode --version

# Check configuration
cat ~/.config/opencode/config.yaml

# Check logs
opencode --debug
```

### Slow responses

This is normal! The Master Agent is thinking with a 405B model.
For faster responses during development, you can:
1. Use a smaller model on the M3 (e.g., 70B or 8B)
2. Configure the orchestrator to use GPU acceleration
3. Use streaming responses (if supported by opencode)

## Development Workflow

```
┌──────────────────────┐
│  You type command    │
│  in opencode TUI     │
└──────┬───────────────┘
       │
       │ HTTP POST
       ▼
┌──────────────────────┐
│  M3 Orchestrator     │
│  receives task       │
└──────┬───────────────┘
       │
       │ Uses Ollama Brain
       ▼
┌──────────────────────┐
│  Master Agent plans  │
│  and executes        │
└──────┬───────────────┘
       │
       │ May spawn workers
       ▼
┌──────────────────────┐
│  Pi Cluster runs     │
│  worker agents       │
└──────┬───────────────┘
       │
       │ Results
       ▼
┌──────────────────────┐
│  You see response    │
│  in opencode TUI     │
└──────────────────────┘
```

## Next Steps

1. [ ] Install opencode
2. [ ] Configure API endpoint
3. [ ] Test connection to orchestrator
4. [ ] Run first command
5. [ ] Build custom workflows
