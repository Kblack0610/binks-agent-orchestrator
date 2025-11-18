"""
Kubectl Tool - The "Hands" for managing the Kubernetes cluster
"""
import subprocess
from typing import Optional
from crewai.tools import tool


@tool("run_kubectl")
def run_kubectl(command: str, namespace: Optional[str] = None) -> str:
    """
    Execute a kubectl command on the cluster.

    This is the primary tool for the Master Agent to interact with the Pi Cluster.

    Args:
        command: The kubectl command to run (without the 'kubectl' prefix)
                 Example: "get pods", "apply -f manifest.yaml"
        namespace: Optional namespace to use (adds -n flag)

    Returns:
        The stdout from the kubectl command, or error message if it fails

    Examples:
        run_kubectl("get pods")
        run_kubectl("get pods", namespace="ai-agents")
        run_kubectl("describe deployment placemyparents")
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


@tool("get_cluster_status")
def get_cluster_status() -> str:
    """
    Get a quick overview of the cluster status.

    Returns information about nodes, namespaces, and key resources.
    """
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
