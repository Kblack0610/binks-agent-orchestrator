#!/bin/bash
#
# Ollama Installation Script
# Installs Ollama for local LLM inference
# Source: https://ollama.ai
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
echo "  Ollama Installation Script"
echo "========================================"
echo ""

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Linux*)     PLATFORM=linux;;
    Darwin*)    PLATFORM=macos;;
    *)          PLATFORM=unknown;;
esac

if [ "$PLATFORM" = "unknown" ]; then
    log_error "Unsupported operating system: $OS"
    echo "Please install Ollama manually from: https://ollama.ai"
    exit 1
fi

log_info "Detected platform: $PLATFORM"

# Check if already installed
if command -v ollama &> /dev/null; then
    log_warn "Ollama is already installed!"
    echo "Current version: $(ollama --version)"
    echo ""
    read -p "Reinstall/update? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 0
    fi
fi

# Install based on platform
if [ "$PLATFORM" = "linux" ]; then
    echo ""
    log_info "Installing Ollama for Linux..."
    curl -fsSL https://ollama.ai/install.sh | sh
elif [ "$PLATFORM" = "macos" ]; then
    if command -v brew &> /dev/null; then
        echo ""
        log_info "Installing Ollama via Homebrew..."
        brew install ollama
    else
        echo ""
        log_info "Installing Ollama for macOS..."
        curl -fsSL https://ollama.ai/install.sh | sh
    fi
fi

# Verify installation
if command -v ollama &> /dev/null; then
    echo ""
    log_info "Ollama installed successfully!"
    echo ""
    echo "Version: $(ollama --version)"
    echo ""
    echo "Next steps:"
    echo "  1. Start Ollama server: ollama serve"
    echo "  2. Pull a model: ollama pull llama3.1:8b"
    echo "  3. Run interactive: ollama run llama3.1:8b"
    echo ""
else
    log_error "Installation may have failed."
    exit 1
fi

# Suggest popular models
echo "========================================"
echo "  Popular Models"
echo "========================================"
echo ""
echo "  ollama pull llama3.1:8b      # Meta's Llama 3.1 (8B)"
echo "  ollama pull mistral          # Mistral 7B"
echo "  ollama pull codellama        # Code-focused Llama"
echo "  ollama pull qwen2.5:7b       # Alibaba Qwen 2.5"
echo ""

# Optional: Pull default model
read -p "Pull llama3.1:8b now? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    log_info "Pulling llama3.1:8b (this may take a while)..."
    ollama pull llama3.1:8b
fi

echo ""
echo "========================================"
echo "  Installation Complete!"
echo "========================================"
echo ""
echo "To start Ollama server:"
echo "  ollama serve"
echo ""
echo "For remote access (e.g., from other machines):"
echo "  OLLAMA_HOST=0.0.0.0:11434 ollama serve"
