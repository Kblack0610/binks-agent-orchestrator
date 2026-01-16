#!/bin/bash
# Restart Ollama on the Mac server (192.168.1.4) with custom settings
# Usage: ./scripts/ollama-restart.sh [keep_alive_duration]
#
# Options:
#   keep_alive_duration - How long to keep models in memory (default: 24h)
#                         Examples: 5m, 1h, 24h, -1 (never unload)

set -e

KEEP_ALIVE="${1:-24h}"
HOST="192.168.1.4"

echo "Connecting to Ollama server at $HOST..."
echo "Setting OLLAMA_KEEP_ALIVE=$KEEP_ALIVE"

# Stop existing Ollama process
ssh "$HOST" "pkill -f 'ollama serve' 2>/dev/null || true"
sleep 2

# Start Ollama with custom settings
# - OLLAMA_HOST=0.0.0.0 allows external connections
# - OLLAMA_KEEP_ALIVE sets how long models stay loaded
ssh "$HOST" "nohup env OLLAMA_KEEP_ALIVE=$KEEP_ALIVE OLLAMA_HOST=0.0.0.0 ollama serve > /tmp/ollama.log 2>&1 &"

sleep 3

# Verify it's running
if ssh "$HOST" "curl -s http://localhost:11434/api/tags" | grep -q "models"; then
    echo "✓ Ollama restarted successfully with KEEP_ALIVE=$KEEP_ALIVE"
    echo ""
    echo "Available models:"
    ssh "$HOST" "curl -s http://localhost:11434/api/tags | jq -r '.models[].name' 2>/dev/null || curl -s http://localhost:11434/api/tags"
else
    echo "✗ Failed to restart Ollama"
    exit 1
fi
