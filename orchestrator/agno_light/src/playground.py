"""
AgentOS Playground - Agno Implementation

This uses Agno's built-in AgentOS for the API server and UI.
This is the "Walk" phase - exposing the agent via API.
"""
import os
from agno.os import AgentOS
from agent import create_master_agent
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
    # Create the master agent
    master_agent = create_master_agent()

    # Create AgentOS with the agent
    agent_os = AgentOS(agents=[master_agent])

    # Start the server
    host = os.getenv('AGNO_API_HOST', '0.0.0.0')
    port = int(os.getenv('AGNO_API_PORT', 8000))

    print(f"Starting AgentOS on {host}:{port}")
    print(f"API: http://{host}:{port}/api")
    print(f"UI: http://{host}:{port}")

    agent_os.serve(host=host, port=port)


if __name__ == "__main__":
    main()
