#!/bin/bash
# Binks MCP Launcher Installer
#
# Quick install:
#   curl -sSL https://raw.githubusercontent.com/kblack0610/binks-agent-orchestrator/master/scripts/install.sh | bash
#
# Options:
#   --pre-download    Download common MCPs during install
#   --mcps "list"     Specific MCPs to pre-download (space-separated)
#   --version TAG     Install specific version
#
set -euo pipefail

# Configuration
REPO="${BINKS_REPO:-kblack0610/binks-agent-orchestrator}"
INSTALL_DIR="${BINKS_CACHE:-${HOME}/.binks/bin}"
RAW_URL="https://raw.githubusercontent.com/${REPO}/master/scripts"

# Colors (if terminal supports it)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' NC=''
fi

info() { echo -e "${BLUE}==>${NC} $*"; }
success() { echo -e "${GREEN}==>${NC} $*"; }
warn() { echo -e "${YELLOW}Warning:${NC} $*"; }
error() { echo -e "${RED}Error:${NC} $*" >&2; }

# Parse arguments
PRE_DOWNLOAD=""
MCPS=""
VERSION="latest"

while [ $# -gt 0 ]; do
    case "$1" in
        --pre-download)
            PRE_DOWNLOAD="yes"
            shift
            ;;
        --mcps)
            MCPS="$2"
            PRE_DOWNLOAD="yes"
            shift 2
            ;;
        --version)
            VERSION="$2"
            shift 2
            ;;
        --help|-h)
            cat << 'EOF'
Binks MCP Launcher Installer

USAGE:
    install.sh [OPTIONS]

OPTIONS:
    --pre-download          Download common MCPs during install
    --mcps "list"          Specific MCPs to pre-download (space-separated)
    --version TAG          Install specific version (default: latest)
    --help                 Show this help

EXAMPLES:
    # Basic install (launcher only)
    curl -sSL https://raw.githubusercontent.com/kblack0610/binks-agent-orchestrator/master/scripts/install.sh | bash

    # Install with common MCPs pre-downloaded
    curl -sSL ... | bash -s -- --pre-download

    # Install specific MCPs
    curl -sSL ... | bash -s -- --mcps "sysinfo-mcp github-gh-mcp"

After installation, add to your PATH:
    export PATH="$HOME/.binks/bin:$PATH"
EOF
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Main installation
main() {
    info "Installing Binks MCP Launcher..."

    # Create install directory
    mkdir -p "$INSTALL_DIR"

    # Download launcher
    local launcher_path="${INSTALL_DIR}/binks-mcp-launcher"
    info "Downloading launcher to ${launcher_path}..."

    if command -v curl &> /dev/null; then
        curl -fsSL "${RAW_URL}/binks-mcp-launcher" -o "$launcher_path"
    elif command -v wget &> /dev/null; then
        wget -q "${RAW_URL}/binks-mcp-launcher" -O "$launcher_path"
    else
        error "curl or wget required"
        exit 1
    fi

    chmod +x "$launcher_path"
    success "Launcher installed!"

    # Pre-download MCPs if requested
    if [ -n "$PRE_DOWNLOAD" ]; then
        local mcps_to_download
        if [ -n "$MCPS" ]; then
            mcps_to_download="$MCPS"
        else
            # Default set of common MCPs
            mcps_to_download="sysinfo-mcp filesystem-mcp git-mcp memory-mcp"
        fi

        info "Pre-downloading MCPs: ${mcps_to_download}"

        for mcp in $mcps_to_download; do
            info "Downloading ${mcp}..."
            BINKS_VERSION="$VERSION" "$launcher_path" --update "$mcp" || {
                warn "Failed to download ${mcp}, will retry on first use"
            }
        done
    fi

    # Check if in PATH
    local in_path=""
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
        in_path="yes"
    fi

    echo ""
    success "Installation complete!"
    echo ""

    if [ -z "$in_path" ]; then
        echo "Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "    export PATH=\"\$HOME/.binks/bin:\$PATH\""
        echo ""
    fi

    echo "Usage:"
    echo "    binks-mcp-launcher <mcp-name>     Run an MCP (downloads on first use)"
    echo "    binks-mcp-launcher --list         List installed MCPs"
    echo "    binks-mcp-launcher --update       Update all MCPs"
    echo ""
    echo "Example .mcp.json configuration:"
    echo '    {'
    echo '      "mcpServers": {'
    echo '        "sysinfo": {'
    echo '          "command": "binks-mcp-launcher",'
    echo '          "args": ["sysinfo-mcp"],'
    echo '          "env": { "BINKS_VERSION": "latest" }'
    echo '        }'
    echo '      }'
    echo '    }'
}

main
