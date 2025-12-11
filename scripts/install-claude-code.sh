#!/bin/bash
#
# Claude Code Installation Script
# Installs Anthropic's official Claude Code CLI
#

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

echo "========================================"
echo "  Claude Code Installation Script"
echo "========================================"
echo ""

# Check for Node.js
if ! command -v node &> /dev/null; then
    log_error "Node.js is not installed!"
    echo ""
    echo "Please install Node.js first:"
    echo "  - Ubuntu/Debian: sudo apt install nodejs npm"
    echo "  - macOS: brew install node"
    echo "  - Or visit: https://nodejs.org/"
    exit 1
fi

NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
if [ "$NODE_VERSION" -lt 18 ]; then
    log_error "Node.js version 18+ required (found: $(node -v))"
    echo "Please upgrade Node.js"
    exit 1
fi

log_info "Node.js version: $(node -v)"

# Check for npm
if ! command -v npm &> /dev/null; then
    log_error "npm is not installed!"
    exit 1
fi

log_info "npm version: $(npm -v)"

# Install Claude Code globally
echo ""
log_info "Installing Claude Code..."
npm install -g @anthropic-ai/claude-code

# Verify installation
if command -v claude &> /dev/null; then
    echo ""
    log_info "Claude Code installed successfully!"
    echo ""
    echo "Version: $(claude --version 2>/dev/null || echo 'installed')"
    echo ""
    echo "Next steps:"
    echo "  1. Set your API key: export ANTHROPIC_API_KEY='your-key'"
    echo "  2. Or add to ~/.bashrc or ~/.zshrc"
    echo "  3. Run: claude --help"
    echo ""
else
    log_error "Installation may have failed. Check npm output above."
    exit 1
fi

# Check for API key
if [ -z "$ANTHROPIC_API_KEY" ]; then
    log_warn "ANTHROPIC_API_KEY environment variable not set"
    echo ""
    echo "To use Claude Code, you'll need an API key from:"
    echo "  https://console.anthropic.com/"
    echo ""
    echo "Add to your shell config:"
    echo '  export ANTHROPIC_API_KEY="your-api-key-here"'
fi

echo "========================================"
echo "  Installation Complete!"
echo "========================================"
