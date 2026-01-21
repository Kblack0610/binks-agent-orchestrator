#!/bin/bash
# E2E Test Runner for Binks Agent
#
# Usage:
#   ./scripts/e2e-test.sh           # Run all tests
#   ./scripts/e2e-test.sh --quick   # Skip E2E, just unit tests
#   ./scripts/e2e-test.sh --e2e     # Only E2E tests (skip unit tests)

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Config
OLLAMA_URL="${OLLAMA_URL:-http://localhost:11434}"
OLLAMA_MODEL="${OLLAMA_MODEL:-llama3.1:8b}"

# Parse args
SKIP_UNIT=false
SKIP_E2E=false
for arg in "$@"; do
    case $arg in
        --quick)
            SKIP_E2E=true
            ;;
        --e2e)
            SKIP_UNIT=true
            ;;
    esac
done

echo "=== Binks Agent E2E Test Runner ==="
echo ""

# 1. Check prerequisites
echo "Checking prerequisites..."

# Check Ollama
if curl -s "$OLLAMA_URL/api/tags" > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Ollama accessible at $OLLAMA_URL"
else
    if [ "$SKIP_E2E" = false ]; then
        echo -e "${RED}✗${NC} Ollama not running at $OLLAMA_URL"
        echo "  Start with: ollama serve"
        echo "  Or run with --quick to skip E2E tests"
        exit 1
    else
        echo -e "${YELLOW}!${NC} Ollama not running (skipping E2E tests)"
    fi
fi

# Check model availability (only if we need E2E)
if [ "$SKIP_E2E" = false ]; then
    if curl -s "$OLLAMA_URL/api/tags" | grep -q "$(echo $OLLAMA_MODEL | cut -d: -f1)"; then
        echo -e "${GREEN}✓${NC} Model $OLLAMA_MODEL available"
    else
        echo -e "${YELLOW}!${NC} Model $OLLAMA_MODEL may not be available"
        echo "  Pull with: ollama pull $OLLAMA_MODEL"
    fi
fi

# 2. Build workspace
echo ""
echo "Building workspace..."
cargo build --workspace
echo -e "${GREEN}✓${NC} Workspace built"

# 3. Run unit tests (unless skipped)
if [ "$SKIP_UNIT" = false ]; then
    echo ""
    echo "Running unit tests..."
    cargo test --workspace
    echo -e "${GREEN}✓${NC} Unit tests passed"
fi

# 4. Run E2E tests (unless skipped)
if [ "$SKIP_E2E" = false ]; then
    echo ""
    echo "Running E2E tests..."
    cd agent
    OLLAMA_URL="$OLLAMA_URL" OLLAMA_MODEL="$OLLAMA_MODEL" \
        cargo test --test e2e -- --include-ignored --test-threads=1 --nocapture
    cd ..
    echo -e "${GREEN}✓${NC} E2E tests passed"

    # 5. Run health check as final verification
    echo ""
    echo "Running agent health check..."
    ./target/debug/agent health --all
fi

echo ""
echo -e "${GREEN}=== All tests passed ===${NC}"
