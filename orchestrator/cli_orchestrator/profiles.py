"""
Agent Profiles - Define agent personas and their configurations

Profiles map SuperClaude-style personas to specific runner configurations,
enabling multi-agent orchestration with specialized roles.
"""
from dataclasses import dataclass, field
from typing import Optional, Dict, Any, List, TYPE_CHECKING
from enum import Enum
from pathlib import Path

# Handle imports for both module and standalone usage
try:
    from .runners.base import CLIRunner
except ImportError:
    from runners.base import CLIRunner


class AgentRole(Enum):
    """Common agent roles for orchestration."""
    ARCHITECT = "architect"
    IMPLEMENTER = "implementer"
    REVIEWER = "reviewer"
    TESTER = "tester"
    RESEARCHER = "researcher"
    ANALYST = "analyst"
    DESIGNER = "designer"
    DEBUGGER = "debugger"
    DOCUMENTER = "documenter"
    CUSTOM = "custom"


@dataclass
class AgentProfile:
    """
    Defines an agent's persona, capabilities, and configuration.

    Profiles can be based on SuperClaude commands or fully custom.
    """
    name: str
    role: AgentRole
    runner: CLIRunner

    # SuperClaude profile (if using Claude with SuperClaude)
    superclaude_command: Optional[str] = None

    # System prompt for non-SuperClaude agents
    system_prompt: Optional[str] = None

    # Description of what this agent does
    description: str = ""

    # Tags for filtering/routing
    tags: List[str] = field(default_factory=list)

    # Additional configuration
    config: Dict[str, Any] = field(default_factory=dict)

    def format_prompt(self, task: str) -> str:
        """
        Format a task into a full prompt for this agent.

        For Claude with SuperClaude, prepends the command.
        For others, wraps with system prompt if available.
        """
        if self.supercloud_command:
            return f"{self.supercloud_command} {task}"

        if self.system_prompt:
            return f"{self.system_prompt}\n\nTask: {task}"

        return task

    @property
    def supercloud_command(self) -> Optional[str]:
        """Alias for superClaude command (handles typo)."""
        return self.superClaude_command if hasattr(self, 'superClaude_command') else self.supercloud_command_internal

    @supercloud_command.setter
    def supercloud_command(self, value):
        self.supercloud_command_internal = value

    def __post_init__(self):
        self.supercloud_command_internal = self.superClaude_command

    @property
    def superClaude_command(self) -> Optional[str]:
        return self.superClaude_command


# Fix the dataclass - simpler version
@dataclass
class AgentProfile:
    """
    Defines an agent's persona, capabilities, and configuration.
    """
    name: str
    role: AgentRole
    runner: CLIRunner
    sc_command: Optional[str] = None  # SuperClaude command like "/sc:design"
    system_prompt: Optional[str] = None
    description: str = ""
    tags: List[str] = field(default_factory=list)
    config: Dict[str, Any] = field(default_factory=dict)

    def format_prompt(self, task: str) -> str:
        """Format a task into a full prompt for this agent."""
        if self.sc_command:
            return f"{self.sc_command} {task}"

        if self.system_prompt:
            return f"{self.system_prompt}\n\nTask: {task}"

        return task


class ProfileRegistry:
    """
    Registry of available agent profiles.

    Manages creation and lookup of agent profiles.
    """

    def __init__(self):
        self._profiles: Dict[str, AgentProfile] = {}

    def register(self, profile: AgentProfile) -> None:
        """Register a profile."""
        self._profiles[profile.name] = profile

    def get(self, name: str) -> Optional[AgentProfile]:
        """Get a profile by name."""
        return self._profiles.get(name)

    def list_profiles(self) -> List[str]:
        """List all registered profile names."""
        return list(self._profiles.keys())

    def by_role(self, role: AgentRole) -> List[AgentProfile]:
        """Get all profiles with a specific role."""
        return [p for p in self._profiles.values() if p.role == role]

    def by_tag(self, tag: str) -> List[AgentProfile]:
        """Get all profiles with a specific tag."""
        return [p for p in self._profiles.values() if tag in p.tags]


def create_superClaude_profile(
    name: str,
    sc_command: str,
    runner: CLIRunner,
    role: AgentRole = AgentRole.CUSTOM,
    **kwargs
) -> AgentProfile:
    """
    Factory function to create a SuperClaude-based profile.

    Args:
        name: Profile name
        sc_command: SuperClaude command (e.g., "/sc:design")
        runner: The CLI runner to use (typically ClaudeRunner)
        role: Agent role
        **kwargs: Additional profile configuration

    Returns:
        Configured AgentProfile
    """
    return AgentProfile(
        name=name,
        role=role,
        runner=runner,
        sc_command=sc_command,
        **kwargs
    )


# Pre-defined SuperClaude profiles
SUPERCLOUD_PROFILES = {
    "architect": {
        "sc_command": "/sc:design",
        "role": AgentRole.ARCHITECT,
        "description": "System architecture and design specialist",
        "tags": ["planning", "architecture", "design"]
    },
    "implementer": {
        "sc_command": "/sc:implement",
        "role": AgentRole.IMPLEMENTER,
        "description": "Feature implementation with best practices",
        "tags": ["coding", "implementation", "development"]
    },
    "analyzer": {
        "sc_command": "/sc:analyze",
        "role": AgentRole.ANALYST,
        "description": "Code analysis and quality assessment",
        "tags": ["analysis", "quality", "review"]
    },
    "tester": {
        "sc_command": "/sc:test",
        "role": AgentRole.TESTER,
        "description": "Testing and quality assurance",
        "tags": ["testing", "qa", "validation"]
    },
    "researcher": {
        "sc_command": "/sc:research",
        "role": AgentRole.RESEARCHER,
        "description": "Deep research and information gathering",
        "tags": ["research", "investigation", "learning"]
    },
    "troubleshooter": {
        "sc_command": "/sc:troubleshoot",
        "role": AgentRole.DEBUGGER,
        "description": "Problem diagnosis and resolution",
        "tags": ["debugging", "troubleshooting", "fixing"]
    },
    "documenter": {
        "sc_command": "/sc:document",
        "role": AgentRole.DOCUMENTER,
        "description": "Documentation generation",
        "tags": ["documentation", "writing", "explanation"]
    },
    "reviewer": {
        "sc_command": "/sc:analyze",  # Uses analyze for code review
        "role": AgentRole.REVIEWER,
        "description": "Code review and feedback",
        "tags": ["review", "feedback", "quality"]
    }
}


def create_default_registry(claude_runner: CLIRunner) -> ProfileRegistry:
    """
    Create a registry with default SuperClaude profiles.

    Args:
        claude_runner: A configured ClaudeRunner instance

    Returns:
        ProfileRegistry with default profiles
    """
    registry = ProfileRegistry()

    for name, config in SUPERCLOUD_PROFILES.items():
        profile = AgentProfile(
            name=name,
            role=config["role"],
            runner=claude_runner,
            sc_command=config["sc_command"],
            description=config["description"],
            tags=config["tags"]
        )
        registry.register(profile)

    return registry
