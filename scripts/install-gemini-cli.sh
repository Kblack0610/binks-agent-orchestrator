#!/bin/bash
#
# Gemini CLI Installation Script
# Installs Google's official Gemini CLI
# Source: https://github.com/google-gemini/gemini-cli
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
echo "  Gemini CLI Installation Script"
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
if [ "$NODE_VERSION" -lt 20 ]; then
    log_error "Node.js version 20+ required (found: $(node -v))"
    echo ""
    echo "To upgrade Node.js:"
    echo "  - Using nvm: nvm install 20 && nvm use 20"
    echo "  - Using brew: brew upgrade node"
    echo "  - Or download from: https://nodejs.org/"
    exit 1
fi

log_info "Node.js version: $(node -v)"

# Check for npm
if ! command -v npm &> /dev/null; then
    log_error "npm is not installed!"
    exit 1
fi

log_info "npm version: $(npm -v)"

# Installation method selection
INSTALL_METHOD="${1:-npm}"  # Default to npm, can pass 'brew' as argument

if [ "$INSTALL_METHOD" = "brew" ]; then
    # Homebrew installation
    if ! command -v brew &> /dev/null; then
        log_error "Homebrew is not installed!"
        echo "Install Homebrew first: https://brew.sh"
        exit 1
    fi

    echo ""
    log_info "Installing Gemini CLI via Homebrew..."
    brew install gemini-cli
else
    # NPM installation (default)
    echo ""
    log_info "Installing Gemini CLI via npm..."
    npm install -g @google/gemini-cli
fi

# Verify installation
if command -v gemini &> /dev/null; then
    echo ""
    log_info "Gemini CLI installed successfully!"
    echo ""
    echo "Version: $(gemini --version 2>/dev/null || echo 'installed')"
    echo ""
    echo "Next steps:"
    echo "  1. Get your API key from: https://aistudio.google.com/apikey"
    echo "  2. Set your API key: export GEMINI_API_KEY='your-key'"
    echo "  3. Or add to ~/.bashrc or ~/.zshrc"
    echo "  4. Run: gemini --help"
    echo ""
else
    log_error "Installation may have failed. Check output above."
    exit 1
fi

# Check for API key
if [ -z "$GEMINI_API_KEY" ] && [ -z "$GOOGLE_API_KEY" ]; then
    log_warn "GEMINI_API_KEY or GOOGLE_API_KEY environment variable not set"
    echo ""
    echo "To use Gemini CLI, you'll need an API key from:"
    echo "  https://aistudio.google.com/apikey"
    echo ""
    echo "Add to your shell config:"
    echo '  export GEMINI_API_KEY="your-api-key-here"'
fi

echo "========================================"
echo "  Installation Complete!"
echo "========================================"
echo ""
echo "Usage:"
echo "  gemini            # Start interactive session"
echo "  gemini 'prompt'   # Quick query"
echo ""
echo "Release channels (optional):"
echo "  npm install -g @google/gemini-cli@preview  # Weekly preview"
echo "  npm install -g @google/gemini-cli@nightly  # Daily builds"
