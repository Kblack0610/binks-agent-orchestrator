# Binks Clients

Remote clients for the Binks Orchestrator API.

## Structure

```
client/
├── python/
│   └── cli.py       # Remote CLI (connects to server)
├── rust/            # (future)
└── web/             # (future)
```

**For local/direct usage:** Use `orchestrator/agno/src/agent.py`

## Python CLI (Remote)

Connects to the Binks Orchestrator API server.

### Usage

```bash
# Interactive mode
python cli.py --host 192.168.1.100

# Single command
python cli.py --host 192.168.1.100 "Check cluster status"

# Health check
python cli.py --health
```

### Configuration

```bash
# Via environment variables
export BINKS_HOST=192.168.1.100
export BINKS_PORT=8000
python cli.py

# Via CLI flags
python cli.py --host 192.168.1.100 --port 8000
```

## Local CLI

For direct usage without a server, use the agent directly:

```bash
cd orchestrator/agno
python src/agent.py
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/invoke` | POST | Send task to agent |
