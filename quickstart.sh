#!/bin/bash
# Global AI - Quick Start Script
# This script helps you get started with the Global AI system

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

print_header() {
    echo -e "\n${GREEN}================================================${NC}"
    echo -e "${GREEN}$1${NC}"
    echo -e "${GREEN}================================================${NC}\n"
}

print_info() {
    echo -e "${YELLOW}ℹ${NC}  $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

check_command() {
    if command -v $1 &> /dev/null; then
        print_success "$1 is installed"
        return 0
    else
        print_error "$1 is NOT installed"
        return 1
    fi
}

print_header "Global AI - Quick Start"

echo "This script will help you get started with your Global AI system."
echo "It will check prerequisites and guide you through the setup."
echo ""

# Detect which component we're setting up
echo "Which component are you setting up?"
echo "1) M3 Orchestrator (AI Control Plane)"
echo "2) Pi Cluster (Kubernetes Compute Plane)"
echo "3) Client (Laptop/Desktop Interface)"
read -p "Enter choice (1-3): " choice

case $choice in
    1)
        print_header "Setting up M3 Orchestrator"

        # Check prerequisites
        print_info "Checking prerequisites..."

        check_command python3 || exit 1
        check_command git || exit 1

        if check_command ollama; then
            print_success "Ollama is installed"
        else
            print_error "Ollama is not installed"
            print_info "Install with: brew install ollama"
            exit 1
        fi

        if check_command kubectl; then
            print_success "kubectl is installed"

            # Test kubectl connection
            if kubectl cluster-info &> /dev/null; then
                print_success "kubectl can connect to cluster"
            else
                print_error "kubectl cannot connect to cluster"
                print_info "You need to copy kubeconfig from your master pi"
                print_info "Run this on your master pi: cat ~/.kube/config"
                print_info "Then save it to ~/.kube/config on this machine"
            fi
        else
            print_error "kubectl is not installed"
            print_info "Install with: brew install kubectl"
            exit 1
        fi

        # Navigate to orchestrator directory
        cd "$(dirname "$0")/orchestrator"

        # Set up Python environment
        print_info "Setting up Python virtual environment..."
        if [ ! -d "venv" ]; then
            python3 -m venv venv
            print_success "Virtual environment created"
        else
            print_info "Virtual environment already exists"
        fi

        source venv/bin/activate

        # Install dependencies
        print_info "Installing Python dependencies..."
        pip install -q --upgrade pip
        pip install -q -r requirements/base.txt
        print_success "Dependencies installed"

        # Set up environment
        if [ ! -f ".env" ]; then
            print_info "Creating .env file..."
            cp .env.example .env
            print_success ".env created - please edit it with your settings"
            print_info "Edit .env with: nano .env"
        else
            print_info ".env already exists"
        fi

        # Check if Ollama is running
        print_info "Checking if Ollama is running..."
        if curl -s --max-time 2 http://localhost:11434/api/version > /dev/null 2>&1; then
            print_success "Ollama is running"

            # Check for models
            print_info "Available models:"
            ollama list
        else
            print_error "Ollama is not running"
            print_info "Start Ollama with: ollama serve"
            print_info "Then pull a model with: ollama pull llama3.1:8b"
        fi

        print_header "Setup Complete!"
        echo "Next steps:"
        echo "1. Make sure Ollama is running: ollama serve"
        echo "2. Pull a model: ollama pull llama3.1:8b"
        echo "3. Edit .env file: nano .env"
        echo "4. Test locally: python src/main.py"
        echo "5. Start API server: python src/api/server.py"
        echo ""
        echo "For detailed instructions, see: SETUP.md"
        ;;

    2)
        print_header "Setting up Pi Cluster"

        # Check prerequisites
        print_info "Checking prerequisites..."

        check_command kubectl || exit 1

        # Test kubectl connection
        if kubectl cluster-info &> /dev/null; then
            print_success "kubectl can connect to cluster"
        else
            print_error "kubectl cannot connect to cluster"
            print_info "Make sure you're on the master pi or have kubeconfig set up"
            exit 1
        fi

        # Navigate to cluster directory
        cd "$(dirname "$0")/cluster"

        # Deploy core services
        print_info "Deploying core services..."

        kubectl apply -f k8s-manifests/core/namespace.yaml
        print_success "Namespaces created"

        kubectl apply -f k8s-manifests/core/ollama-deployment.yaml
        print_success "Ollama deployment created"

        print_info "Waiting for Ollama pod to be ready..."
        kubectl wait --for=condition=ready pod -l app=ollama -n ai-services --timeout=300s
        print_success "Ollama is ready"

        # Pull model
        print_info "Pulling llama3.1:8b model (this may take a while)..."
        kubectl exec -n ai-services deployment/ollama -- ollama pull llama3.1:8b
        print_success "Model pulled"

        print_header "Setup Complete!"
        echo "Your cluster is ready!"
        echo ""
        echo "Check status with:"
        echo "  kubectl get pods -n ai-services"
        echo "  kubectl get pods -n ai-agents"
        echo ""
        echo "For detailed instructions, see: SETUP.md"
        ;;

    3)
        print_header "Setting up Client"

        # Check prerequisites
        print_info "Checking prerequisites..."

        check_command python3 || exit 1

        # Navigate to client directory
        cd "$(dirname "$0")/client"

        # Install dependencies
        print_info "Installing Python dependencies..."
        pip install -q requests pyyaml
        print_success "Dependencies installed"

        # Configure API endpoint
        print_info "Configuring API endpoint..."
        echo ""
        echo "What is the IP address of your M3 Ultra?"
        read -p "M3 IP address: " m3_ip

        # Update config file
        sed -i.bak "s/localhost/$m3_ip/g" config/api-endpoints.yaml
        print_success "Configuration updated"

        # Test connection
        print_info "Testing connection to orchestrator..."
        if curl -s --max-time 5 "http://$m3_ip:8000/health" > /dev/null 2>&1; then
            print_success "Connected to orchestrator!"

            # Show health
            echo ""
            echo "Orchestrator status:"
            curl -s "http://$m3_ip:8000/health" | python3 -m json.tool
        else
            print_error "Could not connect to orchestrator"
            print_info "Make sure the orchestrator is running on your M3"
            print_info "Check with: curl http://$m3_ip:8000/health"
        fi

        print_header "Setup Complete!"
        echo "Your client is ready!"
        echo ""
        echo "Try it out:"
        echo "  python src/simple_client.py --health"
        echo "  python src/simple_client.py --cluster"
        echo "  python src/simple_client.py  # Interactive mode"
        echo ""
        echo "For detailed instructions, see: SETUP.md"
        ;;

    *)
        print_error "Invalid choice"
        exit 1
        ;;
esac
