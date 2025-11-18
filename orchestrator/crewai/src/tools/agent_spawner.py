"""
Agent Spawner Tool - Spawns worker agents as Kubernetes Jobs
"""
import os
import subprocess
import tempfile
import yaml
from typing import Dict, Optional
from crewai.tools import tool


@tool("spawn_worker_agent")
def spawn_worker_agent(
    agent_type: str,
    task_params: Dict[str, str],
    namespace: str = "ai-agents"
) -> str:
    """
    Spawn a worker agent as a Kubernetes Job on the cluster.

    This allows the Master Agent to delegate work to specialized agents
    that run on the Pi Cluster.

    Args:
        agent_type: Type of agent to spawn (e.g., "code-reviewer", "deployer", "tester")
        task_params: Dictionary of parameters to pass to the agent
                     Example: {"repo_url": "https://...", "branch": "main"}
        namespace: Kubernetes namespace to spawn the job in

    Returns:
        Success message with job name, or error message

    Example:
        spawn_worker_agent(
            agent_type="code-reviewer",
            task_params={"repo_url": "https://github.com/user/repo", "task_id": "123"}
        )
    """
    try:
        # Load the job template for this agent type
        template_path = f"../../cluster/k8s-manifests/agents/{agent_type}-job.yaml"

        # Read the template
        with open(template_path, 'r') as f:
            job_yaml = f.read()

        # Replace template variables with actual values
        for key, value in task_params.items():
            placeholder = "{{ " + key.upper() + " }}"
            job_yaml = job_yaml.replace(placeholder, value)

        # Generate unique job name
        import time
        job_name = f"{agent_type}-{int(time.time())}"
        job_yaml = job_yaml.replace("name: code-reviewer-agent", f"name: {job_name}")

        # Write to temp file
        with tempfile.NamedTemporaryFile(mode='w', suffix='.yaml', delete=False) as temp_file:
            temp_file.write(job_yaml)
            temp_path = temp_file.name

        try:
            # Apply the job
            result = subprocess.run(
                ["kubectl", "apply", "-f", temp_path, "-n", namespace],
                capture_output=True,
                text=True,
                timeout=10
            )

            if result.returncode == 0:
                return f"Successfully spawned {agent_type} agent: {job_name}\n{result.stdout}"
            else:
                return f"Error spawning agent:\n{result.stderr}"

        finally:
            # Clean up temp file
            os.unlink(temp_path)

    except FileNotFoundError:
        return f"Error: Agent template not found for type '{agent_type}'"
    except Exception as e:
        return f"Error spawning agent: {str(e)}"


@tool("check_agent_status")
def check_agent_status(job_name: str, namespace: str = "ai-agents") -> str:
    """
    Check the status of a spawned worker agent job.

    Args:
        job_name: Name of the Kubernetes job
        namespace: Namespace the job is running in

    Returns:
        Job status and logs
    """
    try:
        # Get job status
        status_result = subprocess.run(
            ["kubectl", "get", "job", job_name, "-n", namespace, "-o", "yaml"],
            capture_output=True,
            text=True,
            timeout=10
        )

        if status_result.returncode != 0:
            return f"Error: Job '{job_name}' not found"

        # Parse the YAML to get status
        job_data = yaml.safe_load(status_result.stdout)
        status = job_data.get('status', {})

        # Get pod logs
        logs_result = subprocess.run(
            ["kubectl", "logs", f"job/{job_name}", "-n", namespace, "--tail=50"],
            capture_output=True,
            text=True,
            timeout=10
        )

        return f"""Job Status:
Succeeded: {status.get('succeeded', 0)}
Failed: {status.get('failed', 0)}
Active: {status.get('active', 0)}

Recent Logs:
{logs_result.stdout if logs_result.returncode == 0 else 'No logs available'}
"""

    except Exception as e:
        return f"Error checking agent status: {str(e)}"
