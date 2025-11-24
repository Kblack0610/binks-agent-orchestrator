#!/usr/bin/env python3
"""
Simple Python Client for Binks Orchestrator

CLI client for interacting with the Binks Orchestrator API.
"""
import sys
import argparse
import requests
import yaml
from pathlib import Path


class BinksClient:
    """Simple client for the Binks Orchestrator API."""

    def __init__(self, base_url: str):
        """
        Initialize the client.

        Args:
            base_url: Base URL of the orchestrator API (e.g., "http://192.168.1.100:8000")
        """
        self.base_url = base_url.rstrip('/')

    def health_check(self) -> dict:
        """Check the health of the orchestrator."""
        response = requests.get(f"{self.base_url}/health")
        response.raise_for_status()
        return response.json()

    def invoke(self, task: str, context: dict = None) -> dict:
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

        response = requests.post(
            f"{self.base_url}/invoke",
            json=payload,
            timeout=300  # 5 minute timeout for long-running tasks
        )
        response.raise_for_status()
        return response.json()

    def cluster_status(self) -> dict:
        """Get cluster status."""
        response = requests.post(f"{self.base_url}/cluster/status")
        response.raise_for_status()
        return response.json()


def load_config(environment: str = "local") -> str:
    """Load configuration from YAML file."""
    config_file = Path(__file__).parent.parent / "config" / "api-endpoints.yaml"

    with open(config_file, 'r') as f:
        config = yaml.safe_load(f)

    env_config = config['environments'].get(environment)
    if not env_config:
        raise ValueError(f"Environment '{environment}' not found in config")

    return f"{env_config['protocol']}://{env_config['host']}:{env_config['port']}"


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(description="Binks Orchestrator Client")
    parser.add_argument(
        '--env',
        default='local',
        help='Environment to use (default: local)'
    )
    parser.add_argument(
        '--health',
        action='store_true',
        help='Check orchestrator health and exit'
    )
    parser.add_argument(
        '--cluster',
        action='store_true',
        help='Get cluster status and exit'
    )
    parser.add_argument(
        'task',
        nargs='?',
        help='Task to send to the agent'
    )

    args = parser.parse_args()

    # Load configuration
    try:
        base_url = load_config(args.env)
    except Exception as e:
        print(f"Error loading configuration: {e}", file=sys.stderr)
        return 1

    # Initialize client
    client = BinksClient(base_url)

    try:
        # Health check
        if args.health:
            health = client.health_check()
            print("Orchestrator Health:")
            for key, value in health.items():
                print(f"  {key}: {value}")
            return 0

        # Cluster status
        if args.cluster:
            status = client.cluster_status()
            print("Cluster Status:")
            print(status.get('status', 'No status available'))
            return 0

        # Interactive mode if no task provided
        if not args.task:
            print("=" * 60)
            print("Binks Client - Interactive Mode")
            print(f"Connected to: {base_url}")
            print("=" * 60)
            print("\nType your tasks or 'quit' to exit.\n")

            while True:
                try:
                    task = input("You: ")
                    if task.lower() in ['quit', 'exit', 'q']:
                        print("Goodbye!")
                        break

                    if not task.strip():
                        continue

                    print("\nThinking...\n")
                    result = client.invoke(task)

                    if result['success']:
                        print("Agent:")
                        print(result['result'])
                    else:
                        print(f"Error: {result.get('error', 'Unknown error')}")

                    print("\n" + "-" * 60 + "\n")

                except KeyboardInterrupt:
                    print("\n\nGoodbye!")
                    break

        else:
            # Single task mode
            print(f"Sending task to {base_url}...\n")
            result = client.invoke(args.task)

            if result['success']:
                print(result['result'])
                return 0
            else:
                print(f"Error: {result.get('error', 'Unknown error')}", file=sys.stderr)
                return 1

    except requests.exceptions.ConnectionError:
        print(f"Error: Could not connect to orchestrator at {base_url}", file=sys.stderr)
        print("Make sure the orchestrator is running on your M3.", file=sys.stderr)
        return 1
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
