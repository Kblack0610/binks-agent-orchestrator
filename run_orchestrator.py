#!/usr/bin/env python3
"""
Wrapper script to run CLI Orchestrator from anywhere.

Usage:
    python ~/dev/home/binks-agent-orchestrator/run_orchestrator.py --moa "Build API"
    python ~/dev/home/binks-agent-orchestrator/run_orchestrator.py --status
"""
import sys
from pathlib import Path

# Add the orchestrator package to path
repo_root = Path(__file__).parent
sys.path.insert(0, str(repo_root / "orchestrator"))

# Now import and run
from cli_orchestrator.main import main

if __name__ == "__main__":
    main()
