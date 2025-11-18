# Global AI - Makefile
# Common operations for development and deployment

.PHONY: help setup-orchestrator setup-cluster setup-client test-all clean

# Default target
help:
	@echo "Global AI - Available Commands"
	@echo "==============================="
	@echo ""
	@echo "Setup Commands:"
	@echo "  make setup-orchestrator  - Set up M3 orchestrator (run on M3)"
	@echo "  make setup-cluster       - Set up Pi cluster (run on master pi)"
	@echo "  make setup-client        - Set up client (run on laptop)"
	@echo ""
	@echo "Testing Commands:"
	@echo "  make test-orchestrator   - Test orchestrator (run on M3)"
	@echo "  make test-cluster        - Test cluster connection"
	@echo "  make test-client         - Test client (run on laptop)"
	@echo "  make test-all            - Run all tests"
	@echo ""
	@echo "Development Commands:"
	@echo "  make run-orchestrator    - Start orchestrator API server (M3)"
	@echo "  make run-client          - Start interactive client"
	@echo "  make logs-orchestrator   - View orchestrator logs"
	@echo "  make logs-cluster        - View cluster pod logs"
	@echo ""
	@echo "Deployment Commands:"
	@echo "  make deploy-cluster      - Deploy all cluster manifests"
	@echo "  make deploy-ollama       - Deploy Ollama to cluster"
	@echo ""
	@echo "Cleanup Commands:"
	@echo "  make clean               - Clean up generated files"
	@echo "  make clean-cluster       - Remove cluster deployments"

# Setup commands
setup-orchestrator:
	@echo "Setting up M3 Orchestrator..."
	cd orchestrator && \
	python3 -m venv venv && \
	. venv/bin/activate && \
	pip install --upgrade pip && \
	pip install -r requirements/base.txt && \
	cp -n .env.example .env || true
	@echo "✓ Orchestrator setup complete!"
	@echo "  Next: Edit orchestrator/.env with your settings"
	@echo "  Then: make test-orchestrator"

setup-cluster:
	@echo "Setting up Pi Cluster..."
	kubectl apply -f manifests/k8s-manifests/core/namespace.yaml
	kubectl apply -f manifests/k8s-manifests/core/ollama-deployment.yaml
	@echo "Waiting for Ollama to be ready..."
	kubectl wait --for=condition=ready pod -l app=ollama -n ai-services --timeout=300s
	kubectl exec -n ai-services deployment/ollama -- ollama pull llama3.1:8b
	@echo "✓ Cluster setup complete!"
	@echo "  Check status: kubectl get pods -n ai-services"

setup-client:
	@echo "Setting up Client..."
	pip install requests pyyaml
	@echo "✓ Client setup complete!"
	@echo "  Next: Edit client/config/api-endpoints.yaml with M3 IP"
	@echo "  Then: make test-client"

# Testing commands
test-orchestrator:
	@echo "Testing M3 Orchestrator..."
	@echo "1. Checking Ollama..."
	@curl -s http://localhost:11434/api/version > /dev/null && echo "  ✓ Ollama is running" || echo "  ✗ Ollama is NOT running"
	@echo "2. Checking kubectl..."
	@kubectl cluster-info > /dev/null 2>&1 && echo "  ✓ kubectl can access cluster" || echo "  ✗ kubectl CANNOT access cluster"
	@echo "3. Checking Python environment..."
	@test -d orchestrator/venv && echo "  ✓ venv exists" || echo "  ✗ venv does NOT exist"
	@echo ""
	@echo "To test the agent, run: cd orchestrator && source venv/bin/activate && python src/main.py"

test-cluster:
	@echo "Testing Cluster..."
	@echo "Nodes:"
	@kubectl get nodes
	@echo ""
	@echo "AI Services:"
	@kubectl get pods -n ai-services
	@echo ""
	@echo "AI Agents:"
	@kubectl get jobs -n ai-agents

test-client:
	@echo "Testing Client..."
	cd client && python src/simple_client.py --health

test-all: test-orchestrator test-cluster test-client
	@echo ""
	@echo "✓ All tests complete!"

# Development commands
run-orchestrator:
	@echo "Starting Orchestrator API Server..."
	cd orchestrator && . venv/bin/activate && python src/api/server.py

run-client:
	@echo "Starting Interactive Client..."
	cd client && python src/simple_client.py

logs-orchestrator:
	@echo "Orchestrator logs:"
	@tail -f orchestrator/logs/orchestrator.log 2>/dev/null || echo "No logs yet"

logs-cluster:
	@echo "Cluster logs (Ollama service):"
	kubectl logs -n ai-services deployment/ollama --tail=50

# Deployment commands
deploy-cluster:
	@echo "Deploying all cluster manifests..."
	kubectl apply -f manifests/k8s-manifests/core/
	@echo "✓ Cluster manifests deployed"

deploy-ollama:
	@echo "Deploying Ollama to cluster..."
	kubectl apply -f manifests/k8s-manifests/core/ollama-deployment.yaml
	kubectl wait --for=condition=ready pod -l app=ollama -n ai-services --timeout=300s
	@echo "✓ Ollama deployed"

# Cleanup commands
clean:
	@echo "Cleaning up generated files..."
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete 2>/dev/null || true
	find . -type f -name "*.pyo" -delete 2>/dev/null || true
	find . -type d -name "*.egg-info" -exec rm -rf {} + 2>/dev/null || true
	@echo "✓ Cleanup complete"

clean-cluster:
	@echo "Removing cluster deployments..."
	kubectl delete -f manifests/k8s-manifests/core/ --ignore-not-found=true
	@echo "✓ Cluster cleaned"

# Git helpers
git-init:
	@echo "Initializing git repository..."
	git init
	git add .
	git commit -m "Initial commit: Global AI infrastructure"
	@echo "✓ Git initialized"
	@echo "  Next: Add remote with: git remote add origin <your-repo-url>"
	@echo "  Then: git push -u origin main"

# Status check
status:
	@echo "Global AI System Status"
	@echo "======================="
	@echo ""
	@echo "Orchestrator (M3):"
	@curl -s http://localhost:8000/health 2>/dev/null | python3 -m json.tool || echo "  Not running or not accessible"
	@echo ""
	@echo "Cluster:"
	@kubectl get pods --all-namespaces | grep -E "ai-services|ai-agents" || echo "  No AI services running"
