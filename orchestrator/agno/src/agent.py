"""
Master Agent - Agno Implementation

This is a lightweight, high-performance implementation using Agno.
Perfect for infrastructure orchestration with minimal overhead.
"""
import os
from agno.agent import Agent
from agno.models.ollama import Ollama
from dotenv import load_dotenv

# Import custom toolkits
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
from tools.kubectl_tool import KubectlToolkit
from tools.agent_spawner import AgentSpawnerToolkit

# Load environment variables
load_dotenv()


def create_master_agent():
    """
    Create the Master Agent using Agno.

    This agent is lightweight and optimized for infrastructure orchestration.
    """
    # Configure Ollama
    ollama_url = os.getenv('OLLAMA_BASE_URL', 'http://localhost:11434')
    ollama_model = os.getenv('OLLAMA_MODEL', 'llama3.1:405b')

    print(f"Connecting to Ollama at {ollama_url}")
    print(f"Using model: {ollama_model}")

    # Create the agent with toolkits
    agent = Agent(
        name="MasterOrchestrator",
        model=Ollama(id=ollama_model, base_url=ollama_url),
        tools=[
            KubectlToolkit(),
            AgentSpawnerToolkit()
        ],
        instructions=[
            "You are the Global Infrastructure Orchestrator running on a powerful M3 Ultra machine.",
            "You manage a distributed Kubernetes cluster of Raspberry Pis and other compute nodes.",
            "You can spawn specialized worker agents as Kubernetes Jobs to handle specific tasks.",
            "You have access to:",
            "  - run_kubectl: Execute kubectl commands on the cluster",
            "  - get_cluster_status: Quick health check of the cluster",
            "  - spawn_worker_agent: Create a Kubernetes Job for a specialized agent",
            "  - check_agent_status: Monitor spawned worker agents",
            "Always think strategically before acting.",
            "Break complex tasks into smaller steps.",
            "Delegate to worker agents when appropriate."
        ],
        markdown=True,
        show_tool_calls=True,
        debug_mode=False
    )

    return agent


def main():
    """
    Main entry point for testing the Agno Master Agent locally.
    """
    print("=" * 60)
    print("Binks Orchestrator - Agno Implementation (Crawl Phase)")
    print("=" * 60)

    # Create the master agent
    master = create_master_agent()

    # Interactive loop for testing
    print("\nMaster Agent ready. Type your requests or 'quit' to exit.\n")

    while True:
        user_input = input("You: ")

        if user_input.lower() in ['quit', 'exit', 'q']:
            print("Shutting down Master Agent...")
            break

        if not user_input.strip():
            continue

        print("\n" + "-" * 60)

        # Run the agent
        response = master.run(user_input)

        print("-" * 60)
        print("\nAgent Response:")
        print(response.content)
        print("\n")


if __name__ == "__main__":
    main()
