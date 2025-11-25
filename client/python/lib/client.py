"""
Binks Client Library

Shared client for communicating with the Binks Orchestrator API.
Used by CLI, scripts, and other Python clients.
"""
import os
from dataclasses import dataclass
from typing import Optional, Dict, Any, Generator
import requests


@dataclass
class BinksConfig:
    """Configuration for the Binks client."""
    host: str = "localhost"
    port: int = 8000
    protocol: str = "http"
    timeout: int = 300  # 5 minutes for long-running tasks

    @property
    def base_url(self) -> str:
        return f"{self.protocol}://{self.host}:{self.port}"

    @classmethod
    def from_env(cls) -> "BinksConfig":
        """Load configuration from environment variables."""
        return cls(
            host=os.getenv('BINKS_HOST', cls.host),
            port=int(os.getenv('BINKS_PORT', cls.port)),
            protocol=os.getenv('BINKS_PROTOCOL', cls.protocol),
            timeout=int(os.getenv('BINKS_TIMEOUT', cls.timeout))
        )


class BinksClient:
    """
    Client for the Binks Orchestrator API.

    Usage:
        client = BinksClient()  # Uses localhost:8000
        client = BinksClient(BinksConfig(host="192.168.1.100"))

        # Health check
        health = client.health()

        # Invoke agent
        result = client.invoke("Check cluster status")
    """

    def __init__(self, config: Optional[BinksConfig] = None):
        """Initialize the client."""
        self.config = config or BinksConfig.from_env()
        self._session = requests.Session()

    @property
    def base_url(self) -> str:
        return self.config.base_url

    def health(self) -> Dict[str, Any]:
        """Check the health of the orchestrator."""
        response = self._session.get(
            f"{self.base_url}/health",
            timeout=10
        )
        response.raise_for_status()
        return response.json()

    def invoke(self, task: str, context: Optional[Dict[str, str]] = None) -> Dict[str, Any]:
        """
        Send a task to the Master Agent.

        Args:
            task: The task description
            context: Optional context dictionary

        Returns:
            Response from the agent
        """
        payload = {"task": task}
        if context:
            payload["context"] = context

        response = self._session.post(
            f"{self.base_url}/invoke",
            json=payload,
            timeout=self.config.timeout
        )
        response.raise_for_status()
        return response.json()

    def cluster_status(self) -> Dict[str, Any]:
        """Get cluster status."""
        response = self._session.post(
            f"{self.base_url}/cluster/status",
            timeout=30
        )
        response.raise_for_status()
        return response.json()

    def agent_info(self) -> Dict[str, Any]:
        """Get agent information."""
        response = self._session.get(
            f"{self.base_url}/agent/info",
            timeout=10
        )
        response.raise_for_status()
        return response.json()

    def is_available(self) -> bool:
        """Check if the orchestrator is available."""
        try:
            self.health()
            return True
        except requests.exceptions.RequestException:
            return False
