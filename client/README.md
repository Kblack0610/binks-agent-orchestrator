# Binks Clients

All client interfaces for the Binks Orchestrator.

## Structure

```
client/
├── python/              # Python CLI + library
│   ├── cli.py           # Command-line interface
│   └── lib/
│       └── client.py    # Shared client library
├── rust/                # (future)
└── web/                 # (future)
```

## Python CLI

### Installation

```bash
cd client/python
pip install requests python-dotenv
```

### Usage

**Local mode** (runs agent directly, no server needed):
```bash
python cli.py --local
```

**Remote mode** (connects to server):
```bash
python cli.py --host 192.168.1.100
```

**Single command**:
```bash
python cli.py "Check cluster status"
python cli.py --local "List all pods"
```

**Health check**:
```bash
python cli.py --health
```

### Configuration

Via environment variables:
```bash
export BINKS_HOST=192.168.1.100
export BINKS_PORT=8000
python cli.py
```

Or via CLI flags:
```bash
python cli.py --host 192.168.1.100 --port 8000
```

## Client Library

The shared library can be used by any Python code:

```python
from lib.client import BinksClient, BinksConfig

# Default (localhost:8000)
client = BinksClient()

# Custom host
config = BinksConfig(host="192.168.1.100", port=8000)
client = BinksClient(config)

# Check health
if client.is_available():
    health = client.health()
    print(health)

# Invoke agent
result = client.invoke("Check cluster status")
if result['success']:
    print(result['result'])
else:
    print(f"Error: {result['error']}")

# Get cluster status
status = client.cluster_status()

# Get agent info
info = client.agent_info()
```

## Adding New Clients

All clients should use the same HTTP API:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/invoke` | POST | Send task to agent |
| `/cluster/status` | POST | Get K8s status |
| `/agent/info` | GET | Agent info |

### Invoke Request

```json
{
  "task": "Check cluster status",
  "context": {
    "namespace": "default"
  }
}
```

### Invoke Response

```json
{
  "success": true,
  "result": "All pods are running...",
  "error": null
}
```
