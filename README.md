# Binks - Distributed AI Infrastructure System

A decoupled, scalable AI orchestration system with clean client/server separation.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        CLIENTS                               │
│                                                              │
│   Python CLI        Rust CLI (future)      Web UI (future)  │
│   client/python/    client/rust/           client/web/      │
│                                                              │
└────────────────────────────┬────────────────────────────────┘
                             │
                        HTTP/REST
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                    ORCHESTRATOR (Server)                     │
│                    orchestrator/agno/                        │
│                                                              │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                   API Layer                          │   │
│   │              src/api/server.py                       │   │
│   └─────────────────────┬───────────────────────────────┘   │
│                         │                                    │
│   ┌─────────────────────▼───────────────────────────────┐   │
│   │                  Agent Core                          │   │
│   │              src/core/agent.py                       │   │
│   │                                                      │   │
│   │  Tools:                                              │   │
│   │  ├── ShellTools, FileTools (pre-built)              │   │
│   │  └── KubectlToolkit, AgentSpawnerToolkit (custom)   │   │
│   └─────────────────────┬───────────────────────────────┘   │
│                         │                                    │
│                    Ollama (LLM)                              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```
binks/
├── orchestrator/              # SERVER - Agent + API
│   └── agno/
│       ├── src/
│       │   ├── core/          # Agent brain (no I/O)
│       │   │   └── agent.py
│       │   ├── api/           # HTTP API
│       │   │   └── server.py
│       │   └── playground.py  # Agno's built-in UI
│       ├── tools/             # Agent tools
│       └── .env               # Server config
│
├── client/                    # CLIENTS - All interfaces
│   ├── python/
│   │   ├── cli.py             # Python CLI
│   │   └── lib/               # Shared client library
│   │       └── client.py
│   ├── rust/                  # (future)
│   └── web/                   # (future)
│
├── manifests/                 # K8s manifests
└── docs/                      # Documentation
```

## Quick Start

### Option 1: Local Mode (No Server)

Run the CLI directly with the agent (no HTTP):

```bash
cd client/python
python cli.py --local
```

### Option 2: Server Mode

**Start the server (on M3):**
```bash
cd orchestrator/agno
source .venv/bin/activate
python src/api/server.py
```

**Use the CLI (from anywhere):**
```bash
cd client/python
python cli.py --host 192.168.1.100

# Or set via environment
export BINKS_HOST=192.168.1.100
python cli.py
```

## Configuration

### Server (.env)
Location: `orchestrator/agno/.env`

```bash
OLLAMA_BASE_URL=http://192.168.1.4:11434
OLLAMA_MODEL=llama3.1:8b
AGNO_API_HOST=0.0.0.0
AGNO_API_PORT=8000
```

### Client (environment)
```bash
export BINKS_HOST=192.168.1.100
export BINKS_PORT=8000
```

Or use CLI flags:
```bash
python cli.py --host 192.168.1.100 --port 8000
```

## CLI Usage

```bash
# Local mode (no server needed)
python cli.py --local

# Remote mode (needs server running)
python cli.py --host 192.168.1.100

# Single command
python cli.py "Check cluster status"

# Health check
python cli.py --health

# Cluster status
python cli.py --cluster
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/invoke` | POST | Send task to agent |
| `/cluster/status` | POST | Get K8s cluster status |
| `/agent/info` | GET | Agent information |

## Adding New Clients

The `client/python/lib/client.py` provides a reusable client library:

```python
from lib.client import BinksClient, BinksConfig

config = BinksConfig(host="192.168.1.100")
client = BinksClient(config)

# Invoke agent
result = client.invoke("Check cluster status")
print(result['result'])
```

Future clients (Rust, Web, Mobile) will all use the same HTTP API.
