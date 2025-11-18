"""
Master Agent - The "Brain" of the Global AI system

This agent runs on the M3 Ultra and orchestrates all tasks.
It has access to the powerful 405B model via Ollama.
"""
import os
from crewai import Agent, Task, Crew, Process
from langchain_community.llms import Ollama
from dotenv import load_dotenv

# Import tools
from tools.kubectl_tool import run_kubectl, get_cluster_status
from tools.agent_spawner import spawn_worker_agent, check_agent_status

# Load environment variables
load_dotenv()


class MasterAgent:
    """
    The Master Agent orchestrates all tasks in the Global AI system.
    """

    def __init__(self):
        """Initialize the Master Agent with Ollama connection and tools."""
        # Connect to Ollama on M3 Ultra
        ollama_url = os.getenv('OLLAMA_BASE_URL', 'http://localhost:11434')
        ollama_model = os.getenv('OLLAMA_MODEL', 'llama3.1:405b')

        print(f"Connecting to Ollama at {ollama_url}")
        print(f"Using model: {ollama_model}")

        self.llm = Ollama(
            model=ollama_model,
            base_url=ollama_url
        )

        # Define the master agent with all tools
        self.agent = Agent(
            role="Global Infrastructure Orchestrator",
            goal=(
                "Manage the entire Global AI infrastructure, including the Pi Cluster, "
                "applications, and worker agents. Plan complex tasks, delegate to "
                "specialized agents, and ensure all systems are running optimally."
            ),
            backstory=(
                "You are the master AI running on a powerful M3 Ultra machine. "
                "You have access to a distributed Kubernetes cluster of Raspberry Pis "
                "and other compute nodes. You can spawn specialized worker agents as "
                "Kubernetes Jobs to handle specific tasks. You are meticulous, strategic, "
                "and always think before acting."
            ),
            llm=self.llm,
            tools=[
                run_kubectl,
                get_cluster_status,
                spawn_worker_agent,
                check_agent_status
            ],
            allow_delegation=False,  # We handle delegation via spawn_worker_agent
            verbose=True
        )

    def execute_task(self, user_request: str) -> str:
        """
        Execute a task given by the user.

        Args:
            user_request: The task description from the user

        Returns:
            The result of the task execution
        """
        # Create a task from the user request
        task = Task(
            description=user_request,
            expected_output="A detailed report of actions taken and results achieved.",
            agent=self.agent
        )

        # Create a crew with just this agent and task
        crew = Crew(
            agents=[self.agent],
            tasks=[task],
            process=Process.sequential,
            verbose=2
        )

        # Execute the crew
        result = crew.kickoff()

        return str(result)

    def execute_task_with_context(self, user_request: str, context: dict = None) -> str:
        """
        Execute a task with additional context.

        Args:
            user_request: The task description
            context: Additional context (e.g., {"repo_url": "...", "branch": "..."})

        Returns:
            The result of the task execution
        """
        # Build the full task description with context
        full_description = user_request

        if context:
            full_description += "\n\nContext:\n"
            for key, value in context.items():
                full_description += f"- {key}: {value}\n"

        return self.execute_task(full_description)


def main():
    """
    Main entry point for testing the Master Agent locally.
    """
    print("=" * 60)
    print("Binks Orchestrator - Master Agent (Crawl Phase)")
    print("=" * 60)

    # Initialize the master agent
    master = MasterAgent()

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
        result = master.execute_task(user_input)
        print("-" * 60)
        print("\nResult:")
        print(result)
        print("\n")


if __name__ == "__main__":
    main()
