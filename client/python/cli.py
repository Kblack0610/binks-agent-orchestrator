#!/usr/bin/env python3
"""
Binks CLI - Remote Client

Simple client that connects to the Binks Orchestrator API.
For local/direct usage, use: orchestrator/agno/src/agent.py
"""
import sys
import os
import argparse
import requests
from dotenv import load_dotenv

# Load .env from client directory
load_dotenv(os.path.join(os.path.dirname(__file__), '.env'))


class BinksClient:
    """Simple client for the Binks Orchestrator API."""

    def __init__(self, host: str, port: int):
        self.base_url = f"http://{host}:{port}"

    def health(self) -> dict:
        """Check orchestrator health."""
        response = requests.get(f"{self.base_url}/health", timeout=10)
        response.raise_for_status()
        return response.json()

    def invoke(self, task: str) -> dict:
        """Send a task to the agent."""
        response = requests.post(
            f"{self.base_url}/invoke",
            json={"task": task},
            timeout=300
        )
        response.raise_for_status()
        return response.json()

    def is_available(self) -> bool:
        """Check if server is reachable."""
        try:
            self.health()
            return True
        except:
            return False


def interactive_mode(client: BinksClient):
    """Interactive chat mode."""
    print("=" * 60)
    print("Binks CLI - Remote Mode")
    print(f"Connected to: {client.base_url}")
    print("=" * 60)
    print("\nType your tasks or 'quit' to exit.\n")

    while True:
        try:
            user_input = input("You: ").strip()

            if user_input.lower() in ['quit', 'exit', 'q']:
                print("Goodbye!")
                break

            if not user_input:
                continue

            print("\nThinking...\n")
            result = client.invoke(user_input)

            if result.get('success'):
                print("Agent:")
                print(result.get('result', ''))
            else:
                print(f"Error: {result.get('error', 'Unknown error')}")

            print("\n" + "-" * 60 + "\n")

        except KeyboardInterrupt:
            print("\n\nGoodbye!")
            break
        except Exception as e:
            print(f"Error: {e}")


def main():
    parser = argparse.ArgumentParser(
        description="Binks CLI - Remote client for the orchestrator"
    )
    parser.add_argument(
        '--host',
        default=os.getenv('BINKS_HOST', 'localhost'),
        help='Orchestrator host (default: localhost)'
    )
    parser.add_argument(
        '--port',
        type=int,
        default=int(os.getenv('BINKS_PORT', '8000')),
        help='Orchestrator port (default: 8000)'
    )
    parser.add_argument(
        '--health',
        action='store_true',
        help='Check health and exit'
    )
    parser.add_argument(
        'task',
        nargs='?',
        help='Single task to execute'
    )

    args = parser.parse_args()
    client = BinksClient(args.host, args.port)

    try:
        # Health check
        if args.health:
            health = client.health()
            print("Orchestrator Health:")
            for k, v in health.items():
                print(f"  {k}: {v}")
            return 0

        # Single task
        if args.task:
            result = client.invoke(args.task)
            if result.get('success'):
                print(result.get('result', ''))
            else:
                print(f"Error: {result.get('error')}", file=sys.stderr)
                return 1
            return 0

        # Interactive mode
        if not client.is_available():
            print(f"Error: Cannot connect to {client.base_url}")
            print("\nOptions:")
            print("  1. Start server: cd orchestrator/agno && python src/api/server.py")
            print("  2. Use local CLI: cd orchestrator/agno && python src/agent.py")
            return 1

        interactive_mode(client)

    except requests.exceptions.ConnectionError:
        print(f"Error: Cannot connect to {client.base_url}", file=sys.stderr)
        return 1
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
