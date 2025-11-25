"""
Master Agent Core - Pure Agent Logic

This module contains ONLY the agent definition.
No CLI, no I/O, no print statements - just the brain.

Usage:
    from core.agent import create_master_agent
    agent = create_master_agent()
    response = agent.run("your task here")
"""
import os
from dataclasses import dataclass
from typing import Optional

from agno.agent import Agent
from agno.models.ollama import Ollama
from dotenv import load_dotenv

# Pre-built Agno toolkits
from agno.tools.shell import ShellTools
from agno.tools.file import FileTools

# Custom toolkits
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../..'))
from tools.kubectl_tool import KubectlToolkit
from tools.agent_spawner import AgentSpawnerToolkit


@dataclass
class MasterAgentConfig:
    """Configuration for the Master Agent."""
    ollama_url: str = "http://localhost:11434"
    ollama_model: str = "llama3.1:8b"
    agent_name: str = "MasterOrchestrator"
    debug_mode: bool = False

    @classmethod
    def from_env(cls) -> "MasterAgentConfig":
        """Load configuration from environment variables."""
        load_dotenv()
        return cls(
            ollama_url=os.getenv('OLLAMA_BASE_URL', cls.ollama_url),
            ollama_model=os.getenv('OLLAMA_MODEL', cls.ollama_model),
            debug_mode=os.getenv('DEBUG_MODE', 'false').lower() == 'true'
        )


def create_master_agent(config: Optional[MasterAgentConfig] = None) -> Agent:
    """
    Create the Master Agent.

    Args:
        config: Optional configuration. If not provided, loads from environment.

    Returns:
        Configured Agno Agent instance.
    """
    if config is None:
        config = MasterAgentConfig.from_env()

    agent = Agent(
        name=config.agent_name,
        model=Ollama(id=config.ollama_model, host=config.ollama_url),
        tools=[
            # Pre-built tools (code editing & general operations)
            ShellTools(),
            FileTools(),

            # Custom tools (infrastructure orchestration)
            KubectlToolkit(),
            AgentSpawnerToolkit()
        ],
        instructions=[
            "You are the Global Infrastructure Orchestrator.",
            "You manage a distributed Kubernetes cluster and can perform code operations.",
            "",
            "You have access to these tools:",
            "",
            "CODE EDITING (Pre-built):",
            "  - ShellTools: Run any shell command (git, scripts, builds, tests)",
            "  - FileTools: Read, write, and list files in any directory",
            "",
            "INFRASTRUCTURE (Custom):",
            "  - run_kubectl: Execute kubectl commands on the cluster",
            "  - get_cluster_status: Quick health check of the cluster",
            "  - spawn_worker_agent: Create a Kubernetes Job for a specialized agent",
            "  - check_agent_status: Monitor spawned worker agents",
            "",
            "Always think strategically before acting.",
            "Break complex tasks into smaller steps.",
            "For code tasks, use ShellTools and FileTools.",
            "For cluster tasks, use kubectl and agent spawner tools.",
        ],
        markdown=True,
        debug_mode=config.debug_mode
    )

    return agent
