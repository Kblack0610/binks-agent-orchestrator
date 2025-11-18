"""
Kubectl Tool - Agno Implementation
Lighter weight version using Agno's toolkit pattern
"""
import subprocess
from typing import Optional
from agno.tools import Toolkit


class KubectlToolkit(Toolkit):
    """Toolkit for managing Kubernetes cluster via kubectl."""

    def __init__(self):
        super().__init__(name="kubectl_toolkit")

    def run_kubectl(self, command: str, namespace: Optional[str] = None) -> str:
        """
        Execute a kubectl command on the cluster.

        Args:
            command: The kubectl command to run (without the 'kubectl' prefix)
                     Example: "get pods", "apply -f manifest.yaml"
            namespace: Optional namespace to use (adds -n flag)

        Returns:
            The stdout from the kubectl command, or error message if it fails
        """
        try:
            # Build the full command
            full_command = ["kubectl"]

            # Add namespace if provided
            if namespace:
                full_command.extend(["-n", namespace])

            # Add the user's command
            full_command.extend(command.split())

            # Execute
            result = subprocess.run(
                full_command,
                capture_output=True,
                text=True,
                timeout=30
            )

            # Return stdout if successful, stderr if failed
            if result.returncode == 0:
                return result.stdout
            else:
                return f"Error (exit code {result.returncode}):\n{result.stderr}"

        except subprocess.TimeoutExpired:
            return "Error: Command timed out after 30 seconds"
        except Exception as e:
            return f"Error executing kubectl: {str(e)}"

    def get_cluster_status(self) -> str:
        """Get a quick overview of the cluster status."""
        try:
            status_parts = []

            # Get node status
            nodes_result = subprocess.run(
                ["kubectl", "get", "nodes", "-o", "wide"],
                capture_output=True,
                text=True,
                timeout=10
            )
            status_parts.append("=== NODES ===")
            status_parts.append(nodes_result.stdout if nodes_result.returncode == 0 else "Error getting nodes")

            # Get all pods
            pods_result = subprocess.run(
                ["kubectl", "get", "pods", "--all-namespaces"],
                capture_output=True,
                text=True,
                timeout=10
            )
            status_parts.append("\n=== PODS ===")
            status_parts.append(pods_result.stdout if pods_result.returncode == 0 else "Error getting pods")

            return "\n".join(status_parts)

        except Exception as e:
            return f"Error getting cluster status: {str(e)}"
