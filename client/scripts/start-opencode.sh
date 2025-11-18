#!/bin/bash
# Start opencode TUI configured to use Binks Orchestrator

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONFIG_FILE="$PROJECT_ROOT/config/api-endpoints.yaml"

# Default environment
ENVIRONMENT="${1:-local}"

# Parse YAML config (simple parsing - could use yq for more complex configs)
if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "Error: Config file not found at $CONFIG_FILE"
    exit 1
fi

# Extract host, port, protocol for the selected environment
# This is a simple implementation - for production use yq or python
HOST=$(grep -A 4 "^  $ENVIRONMENT:" "$CONFIG_FILE" | grep "host:" | awk '{print $2}' | tr -d '"')
PORT=$(grep -A 4 "^  $ENVIRONMENT:" "$CONFIG_FILE" | grep "port:" | awk '{print $2}')
PROTOCOL=$(grep -A 4 "^  $ENVIRONMENT:" "$CONFIG_FILE" | grep "protocol:" | awk '{print $2}' | tr -d '"')

if [[ -z "$HOST" ]] || [[ -z "$PORT" ]]; then
    echo "Error: Could not parse configuration for environment '$ENVIRONMENT'"
    echo "Available environments in $CONFIG_FILE:"
    grep "^  [a-z]" "$CONFIG_FILE"
    exit 1
fi

ORCHESTRATOR_URL="${PROTOCOL}://${HOST}:${PORT}"

echo "=========================================="
echo "Binks Client - OpenCode TUI"
echo "=========================================="
echo "Environment: $ENVIRONMENT"
echo "Orchestrator URL: $ORCHESTRATOR_URL"
echo ""

# Test connection
echo "Testing connection to orchestrator..."
if curl -s --max-time 5 "${ORCHESTRATOR_URL}/health" > /dev/null 2>&1; then
    echo "✓ Connection successful!"

    # Show orchestrator status
    HEALTH=$(curl -s "${ORCHESTRATOR_URL}/health")
    echo "Orchestrator Status:"
    echo "$HEALTH" | python3 -m json.tool 2>/dev/null || echo "$HEALTH"
else
    echo "✗ Warning: Could not connect to orchestrator at $ORCHESTRATOR_URL"
    echo "  Make sure the orchestrator is running on your M3."
    echo "  You can still continue, but requests will fail."
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo ""
echo "Starting opencode TUI..."
echo "=========================================="
echo ""

# TODO: Configure opencode to use the orchestrator API
# This depends on how opencode accepts custom API endpoints
# For now, we'll document the manual steps

echo "NOTE: OpenCode configuration needed!"
echo ""
echo "OpenCode needs to be configured to use your orchestrator."
echo "Add this to your OpenCode config (usually ~/.config/opencode/config.yaml):"
echo ""
echo "api:"
echo "  endpoint: $ORCHESTRATOR_URL"
echo "  type: custom"
echo ""
echo "Then launch opencode normally:"
echo "  opencode"
echo ""

# If opencode supports environment variables for API endpoint:
# export OPENCODE_API_ENDPOINT="$ORCHESTRATOR_URL"
# opencode

# If opencode supports command-line flags:
# opencode --api-endpoint "$ORCHESTRATOR_URL"

# For now, just provide instructions
echo "Press Enter to continue or Ctrl+C to exit..."
read

# Launch opencode (assuming it's installed)
if command -v opencode &> /dev/null; then
    opencode
else
    echo "Error: 'opencode' command not found."
    echo "Please install opencode first:"
    echo "  pip install opencode"
    exit 1
fi
