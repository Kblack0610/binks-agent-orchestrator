#!/usr/bin/env python3
"""
Binks CLI

Command-line interface for the Binks Orchestrator.

Usage:
    # Remote mode (talks to server via HTTP)
    python cli.py --host 192.168.1.100

    # Local mode (runs agent directly, no server needed)
    python cli.py --local

    # Single command
    python cli.py "Check cluster status"

    # Health check
    python cli.py --health
"""
import sys
import os
import argparse

# Add lib to path
sys.path.insert(0, os.path.dirname(__file__))
from lib.client import BinksClient, BinksConfig


def run_remote_mode(client: BinksClient):
    """Interactive mode using remote API."""
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


def run_local_mode():
    """Interactive mode running agent directly (no server needed)."""
    # Import here to avoid loading agent code when not needed
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../orchestrator/agno/src'))
    from core.agent import create_master_agent

    print("=" * 60)
    print("Binks CLI - Local Mode")
    print("=" * 60)
    print("\nInitializing agent...")

    agent = create_master_agent()

    print("Agent ready. Type your tasks or 'quit' to exit.\n")

    while True:
        try:
            user_input = input("You: ").strip()

            if user_input.lower() in ['quit', 'exit', 'q']:
                print("Goodbye!")
                break

            if not user_input:
                continue

            print("\n" + "-" * 60)
            response = agent.run(user_input)
            print("-" * 60)
            print("\nAgent:")
            print(response.content if hasattr(response, 'content') else str(response))
            print("\n")

        except KeyboardInterrupt:
            print("\n\nGoodbye!")
            break
        except Exception as e:
            print(f"Error: {e}")


def main():
    parser = argparse.ArgumentParser(
        description="Binks CLI - Interface for the Binks Orchestrator"
    )
    parser.add_argument(
        '--host',
        default=os.getenv('BINKS_HOST', 'localhost'),
        help='Orchestrator host (default: localhost)'
    )
    parser.add_argument(
        '--port',
        type=int,
        default=int(os.getenv('BINKS_PORT', 8000)),
        help='Orchestrator port (default: 8000)'
    )
    parser.add_argument(
        '--local',
        action='store_true',
        help='Run in local mode (no server needed, runs agent directly)'
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
        help='Single task to execute (then exit)'
    )

    args = parser.parse_args()

    # Local mode - run agent directly
    if args.local:
        if args.task:
            # Single task in local mode
            sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../orchestrator/agno/src'))
            from core.agent import create_master_agent
            agent = create_master_agent()
            response = agent.run(args.task)
            print(response.content if hasattr(response, 'content') else str(response))
        else:
            run_local_mode()
        return 0

    # Remote mode - use API
    config = BinksConfig(host=args.host, port=args.port)
    client = BinksClient(config)

    try:
        # Health check
        if args.health:
            health = client.health()
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

        # Single task
        if args.task:
            result = client.invoke(args.task)
            if result.get('success'):
                print(result.get('result', ''))
            else:
                print(f"Error: {result.get('error', 'Unknown error')}", file=sys.stderr)
                return 1
            return 0

        # Interactive mode
        if not client.is_available():
            print(f"Error: Cannot connect to orchestrator at {client.base_url}")
            print("Options:")
            print("  1. Start the server: cd orchestrator/agno && python src/api/server.py")
            print("  2. Use local mode: python cli.py --local")
            return 1

        run_remote_mode(client)

    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
