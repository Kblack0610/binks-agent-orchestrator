#!/bin/bash
# Build the Playwright MCP sandbox Docker image

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Building binks-playwright-sandbox Docker image..."
docker build -t binks-playwright-sandbox:latest "$SCRIPT_DIR"

echo "Done! Image built: binks-playwright-sandbox:latest"
echo ""
echo "To test the image:"
echo "  echo '{\"jsonrpc\":\"2.0\",\"method\":\"tools/list\",\"id\":1}' | docker run -i --rm binks-playwright-sandbox"
