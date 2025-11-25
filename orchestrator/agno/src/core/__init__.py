"""
Binks Orchestrator Core

The agent brain - no CLI, no I/O, just pure agent logic.
"""
from .agent import create_master_agent, MasterAgentConfig

__all__ = ["create_master_agent", "MasterAgentConfig"]
