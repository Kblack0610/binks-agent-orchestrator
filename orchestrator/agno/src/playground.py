"""
AgentOS Playground - Agno's Built-in Server

Uses Agno's built-in AgentOS for the API server and UI.
Alternative to the custom FastAPI server.
"""
import os
import sys

# Add current directory to path
sys.path.insert(0, os.path.dirname(__file__))

from agno.playground import Playground
from core.agent import create_master_agent
from dotenv import load_dotenv

# Load environment
load_dotenv()


def main():
    """
    Start the AgentOS playground (API server + UI).

    This provides:
    - REST API at http://localhost:8000/api
    - Web UI at http://localhost:8000
    - Agent monitoring and debugging
    """
    print("=" * 60)
    print("Binks Orchestrator - Playground")
    print("=" * 60)
    print("Initializing agent...")

    # Create the master agent
    master_agent = create_master_agent()

    # Create Playground with the agent
    playground = Playground(agents=[master_agent])

    # Start the server
    host = os.getenv('AGNO_API_HOST', '0.0.0.0')
    port = int(os.getenv('AGNO_API_PORT', 8000))

    print(f"\nStarting Playground on {host}:{port}")
    print(f"API: http://{host}:{port}")
    print("=" * 60)

    playground.serve(host=host, port=port)


if __name__ == "__main__":
    main()
