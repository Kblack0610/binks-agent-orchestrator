#!/usr/bin/env python3
"""
Binks Orchestrator - Main Entry Point

This is the entry point for the Master Agent running on the M3 Ultra.
"""
import sys
import os

# Add src to path for imports
sys.path.insert(0, os.path.dirname(__file__))

from agents.master_agent import main

if __name__ == "__main__":
    main()
