#!/bin/bash
#
# Binks Agent Orchestrator - Master Setup Script
# Installs all required CLI tools for the agent orchestrator
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_header() { echo -e "${BLUE}$1${NC}"; }

print_banner() {
    echo -e "${CYAN}"
    echo "╔═══════════════════════════════════════════════════════╗"
    echo "║     Binks Agent Orchestrator - Setup Script           ║"
    echo "║                                                       ║"
    echo "║     Polyglot CLI Orchestrator for AI Agents           ║"
    echo "╚═══════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

check_prerequisites() {
    log_header "Checking Prerequisites..."
    echo ""

    # Check Node.js
    if command -v node &> /dev/null; then
        NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
        if [ "$NODE_VERSION" -ge 20 ]; then
            log_info "Node.js $(node -v) - OK"
        elif [ "$NODE_VERSION" -ge 18 ]; then
            log_warn "Node.js $(node -v) - OK for Claude, but Gemini needs 20+"
        else
            log_error "Node.js $(node -v) - Needs upgrade (18+ required)"
        fi
    else
        log_error "Node.js - Not installed"
    fi

    # Check Python
    if command -v python3 &> /dev/null; then
        log_info "Python $(python3 --version | cut -d' ' -f2) - OK"
    else
        log_error "Python - Not installed"
    fi

    # Check pip
    if command -v pip3 &> /dev/null; then
        log_info "pip $(pip3 --version | cut -d' ' -f2) - OK"
    else
        log_warn "pip - Not installed"
    fi

    echo ""
}

check_installed() {
    log_header "Checking Installed Tools..."
    echo ""

    # Claude Code
    if command -v claude &> /dev/null; then
        log_info "Claude Code - Installed"
    else
        log_warn "Claude Code - Not installed"
    fi

    # Gemini CLI
    if command -v gemini &> /dev/null; then
        log_info "Gemini CLI - Installed"
    else
        log_warn "Gemini CLI - Not installed"
    fi

    # Ollama
    if command -v ollama &> /dev/null; then
        log_info "Ollama - Installed ($(ollama --version 2>/dev/null || echo 'version unknown'))"
    else
        log_warn "Ollama - Not installed"
    fi

    echo ""
}

check_api_keys() {
    log_header "Checking API Keys..."
    echo ""

    if [ -n "$ANTHROPIC_API_KEY" ]; then
        log_info "ANTHROPIC_API_KEY - Set"
    else
        log_warn "ANTHROPIC_API_KEY - Not set (needed for Claude)"
    fi

    if [ -n "$GEMINI_API_KEY" ] || [ -n "$GOOGLE_API_KEY" ]; then
        log_info "GEMINI_API_KEY - Set"
    else
        log_warn "GEMINI_API_KEY - Not set (needed for Gemini)"
    fi

    echo ""
}

install_menu() {
    log_header "Installation Menu"
    echo ""
    echo "Select tools to install:"
    echo ""
    echo "  1) Claude Code      - Anthropic's Claude CLI"
    echo "  2) Gemini CLI       - Google's Gemini CLI"
    echo "  3) Ollama           - Local LLM inference"
    echo "  4) All of the above"
    echo "  5) Python dependencies only"
    echo "  0) Exit"
    echo ""
    read -p "Enter choice [0-5]: " choice
    echo ""

    case $choice in
        1)
            bash "$SCRIPT_DIR/install-claude-code.sh"
            ;;
        2)
            bash "$SCRIPT_DIR/install-gemini-cli.sh"
            ;;
        3)
            bash "$SCRIPT_DIR/install-ollama.sh"
            ;;
        4)
            log_info "Installing all tools..."
            echo ""
            bash "$SCRIPT_DIR/install-claude-code.sh"
            echo ""
            bash "$SCRIPT_DIR/install-gemini-cli.sh"
            echo ""
            bash "$SCRIPT_DIR/install-ollama.sh"
            ;;
        5)
            install_python_deps
            ;;
        0)
            log_info "Exiting..."
            exit 0
            ;;
        *)
            log_error "Invalid choice"
            install_menu
            ;;
    esac
}

install_python_deps() {
    log_header "Installing Python Dependencies..."
    echo ""

    PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
    ORCHESTRATOR_DIR="$PROJECT_ROOT/orchestrator/cli_orchestrator"

    if [ -f "$ORCHESTRATOR_DIR/requirements.txt" ]; then
        pip3 install -r "$ORCHESTRATOR_DIR/requirements.txt"
    else
        # Install minimal requirements
        pip3 install anthropic google-generativeai pytest
    fi

    log_info "Python dependencies installed"
}

print_summary() {
    echo ""
    log_header "═══════════════════════════════════════════════════════"
    log_header "                     Summary"
    log_header "═══════════════════════════════════════════════════════"
    echo ""

    check_installed
    check_api_keys

    echo "To get started:"
    echo ""
    echo "  cd orchestrator/cli_orchestrator"
    echo "  python -m cli_orchestrator.main --check"
    echo "  python -m cli_orchestrator.main --moa 'Write hello world'"
    echo ""
    echo "Documentation:"
    echo "  docs/projects/          - Project ideas"
    echo "  orchestrator/cli_orchestrator/README.md"
    echo ""
}

# Main execution
print_banner

# Parse arguments
case "${1:-}" in
    --check)
        check_prerequisites
        check_installed
        check_api_keys
        exit 0
        ;;
    --claude)
        bash "$SCRIPT_DIR/install-claude-code.sh"
        exit 0
        ;;
    --gemini)
        bash "$SCRIPT_DIR/install-gemini-cli.sh"
        exit 0
        ;;
    --ollama)
        bash "$SCRIPT_DIR/install-ollama.sh"
        exit 0
        ;;
    --all)
        bash "$SCRIPT_DIR/install-claude-code.sh"
        bash "$SCRIPT_DIR/install-gemini-cli.sh"
        bash "$SCRIPT_DIR/install-ollama.sh"
        print_summary
        exit 0
        ;;
    --help|-h)
        echo "Usage: $0 [OPTION]"
        echo ""
        echo "Options:"
        echo "  --check     Check prerequisites and installed tools"
        echo "  --claude    Install Claude Code only"
        echo "  --gemini    Install Gemini CLI only"
        echo "  --ollama    Install Ollama only"
        echo "  --all       Install all tools"
        echo "  --help      Show this help"
        echo ""
        echo "Without options, runs interactive menu."
        exit 0
        ;;
    "")
        # Interactive mode
        check_prerequisites
        check_installed
        check_api_keys
        install_menu
        print_summary
        ;;
    *)
        log_error "Unknown option: $1"
        echo "Use --help for usage information"
        exit 1
        ;;
esac
