"""
CLI Orchestrator - Multi-Agent Conversation Orchestration

Enables headless, back-and-forth conversations between multiple AI agents
using Claude CLI, Gemini, and custom modules.

Features:
- Multi-agent conversations with role-based handoffs
- Conversation history and context management
- Benchmarking across different backends
- Flexible routing based on task type
"""
import json
import time
from dataclasses import dataclass, field
from typing import Optional, Dict, Any, List, Callable
from pathlib import Path
from datetime import datetime
from enum import Enum

# Handle imports for both module and standalone usage
try:
    from .runners.base import CLIRunner, RunnerResult
    from .profiles import AgentProfile, ProfileRegistry, AgentRole
except ImportError:
    from runners.base import CLIRunner, RunnerResult
    from profiles import AgentProfile, ProfileRegistry, AgentRole


@dataclass
class ConversationTurn:
    """A single turn in a multi-agent conversation."""
    agent_name: str
    role: AgentRole
    prompt: str
    response: str
    timestamp: datetime = field(default_factory=datetime.now)
    execution_time: float = 0.0
    backend: str = ""
    model: str = ""
    success: bool = True
    error: Optional[str] = None
    metadata: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict:
        return {
            "agent": self.agent_name,
            "role": self.role.value,
            "prompt": self.prompt,
            "response": self.response,
            "timestamp": self.timestamp.isoformat(),
            "execution_time": self.execution_time,
            "backend": self.backend,
            "success": self.success
        }


@dataclass
class Conversation:
    """Represents a full multi-agent conversation."""
    id: str
    goal: str
    turns: List[ConversationTurn] = field(default_factory=list)
    status: str = "active"
    created_at: datetime = field(default_factory=datetime.now)
    metadata: Dict[str, Any] = field(default_factory=dict)

    def add_turn(self, turn: ConversationTurn) -> None:
        self.turns.append(turn)

    def get_context(self, max_turns: int = 10) -> str:
        """Get conversation context for the next agent."""
        recent = self.turns[-max_turns:] if len(self.turns) > max_turns else self.turns

        context_parts = [f"Goal: {self.goal}\n"]
        for turn in recent:
            context_parts.append(
                f"\n[{turn.agent_name} ({turn.role.value})]:\n{turn.response}\n"
            )

        return "\n".join(context_parts)

    def get_last_response(self) -> Optional[str]:
        """Get the most recent response."""
        if self.turns:
            return self.turns[-1].response
        return None

    def to_dict(self) -> dict:
        return {
            "id": self.id,
            "goal": self.goal,
            "status": self.status,
            "turns": [t.to_dict() for t in self.turns],
            "created_at": self.created_at.isoformat()
        }

    def save(self, path: Path) -> None:
        """Save conversation to JSON file."""
        with open(path, 'w') as f:
            json.dump(self.to_dict(), f, indent=2)


class Orchestrator:
    """
    Main orchestrator for multi-agent conversations.

    Manages agent profiles, conversation flow, and handoffs.
    """

    def __init__(
        self,
        registry: ProfileRegistry,
        working_dir: Optional[Path] = None,
        max_turns: int = 20,
        debug: bool = False
    ):
        self.registry = registry
        self.working_dir = working_dir or Path.cwd()
        self.max_turns = max_turns
        self.debug = debug
        self._conversations: Dict[str, Conversation] = {}

    def create_conversation(self, goal: str) -> Conversation:
        """Start a new conversation with a goal."""
        conv_id = f"conv_{int(time.time())}"
        conversation = Conversation(id=conv_id, goal=goal)
        self._conversations[conv_id] = conversation
        return conversation

    def run_agent(
        self,
        conversation: Conversation,
        agent_name: str,
        task: str,
        include_context: bool = True
    ) -> ConversationTurn:
        """
        Run a single agent turn in a conversation.

        Args:
            conversation: The conversation context
            agent_name: Name of the agent profile to use
            task: The task/prompt for this agent
            include_context: Whether to include conversation history

        Returns:
            ConversationTurn with the result
        """
        profile = self.registry.get(agent_name)
        if not profile:
            raise ValueError(f"Unknown agent profile: {agent_name}")

        # Build full prompt with context
        if include_context and conversation.turns:
            context = conversation.get_context()
            full_prompt = f"{context}\n\nYour task: {task}"
        else:
            full_prompt = task

        # Format with profile's template (e.g., SuperClaude command)
        formatted_prompt = profile.format_prompt(full_prompt)

        if self.debug:
            print(f"\n[Orchestrator] Running {agent_name}...")
            print(f"[Orchestrator] Prompt length: {len(formatted_prompt)} chars")

        # Execute
        start_time = time.time()
        result = profile.runner.run(formatted_prompt)
        execution_time = time.time() - start_time

        # Create turn
        turn = ConversationTurn(
            agent_name=agent_name,
            role=profile.role,
            prompt=task,
            response=result.content,
            execution_time=execution_time,
            backend=result.backend,
            model=result.model,
            success=result.success,
            error=result.error,
            metadata=result.metadata
        )

        conversation.add_turn(turn)
        return turn

    def run_workflow(
        self,
        goal: str,
        workflow: List[tuple[str, str]],
        handoff_template: str = "Based on the above, {task}"
    ) -> Conversation:
        """
        Run a predefined workflow of agent handoffs.

        Args:
            goal: The overall goal
            workflow: List of (agent_name, task_description) tuples
            handoff_template: Template for formatting handoff tasks

        Returns:
            Completed Conversation
        """
        conversation = self.create_conversation(goal)

        for i, (agent_name, task) in enumerate(workflow):
            if self.debug:
                print(f"\n[Workflow] Step {i+1}/{len(workflow)}: {agent_name}")

            # Format task with handoff template if not first turn
            if i > 0 and "{task}" in handoff_template:
                formatted_task = handoff_template.format(task=task)
            else:
                formatted_task = task

            turn = self.run_agent(
                conversation,
                agent_name,
                formatted_task,
                include_context=(i > 0)
            )

            if not turn.success:
                if self.debug:
                    print(f"[Workflow] Agent {agent_name} failed: {turn.error}")
                conversation.status = "failed"
                break

        if conversation.status != "failed":
            conversation.status = "completed"

        return conversation

    def run_loop(
        self,
        goal: str,
        agents: List[str],
        initial_prompt: str,
        stop_condition: Optional[Callable[[Conversation], bool]] = None,
        max_iterations: int = 10
    ) -> Conversation:
        """
        Run agents in a loop until stop condition or max iterations.

        Args:
            goal: The overall goal
            agents: List of agent names to cycle through
            initial_prompt: Starting prompt
            stop_condition: Function that returns True to stop
            max_iterations: Maximum number of full cycles

        Returns:
            Completed Conversation
        """
        conversation = self.create_conversation(goal)

        # First turn with initial prompt
        first_agent = agents[0]
        self.run_agent(conversation, first_agent, initial_prompt, include_context=False)

        # Loop through agents
        iteration = 0
        agent_index = 1

        while iteration < max_iterations:
            # Check stop condition
            if stop_condition and stop_condition(conversation):
                if self.debug:
                    print("[Loop] Stop condition met")
                break

            # Check turn limit
            if len(conversation.turns) >= self.max_turns:
                if self.debug:
                    print("[Loop] Max turns reached")
                break

            # Get next agent
            agent_name = agents[agent_index % len(agents)]

            # Generate handoff prompt based on last response
            last_response = conversation.get_last_response()
            handoff_prompt = f"Review and continue: {last_response[:500]}..."

            turn = self.run_agent(conversation, agent_name, handoff_prompt)

            if not turn.success:
                break

            agent_index += 1
            if agent_index % len(agents) == 0:
                iteration += 1

        conversation.status = "completed"
        return conversation


class WorkflowBuilder:
    """
    Fluent builder for creating agent workflows.

    Usage:
        workflow = (WorkflowBuilder(orchestrator)
            .goal("Build a REST API")
            .then("architect", "Design the API structure")
            .then("implementer", "Implement the endpoints")
            .then("tester", "Write tests")
            .then("reviewer", "Review the implementation")
            .build())
    """

    def __init__(self, orchestrator: Orchestrator):
        self.orchestrator = orchestrator
        self._goal: str = ""
        self._steps: List[tuple[str, str]] = []
        self._handoff_template: str = "Based on the above, {task}"

    def goal(self, goal: str) -> "WorkflowBuilder":
        self._goal = goal
        return self

    def then(self, agent_name: str, task: str) -> "WorkflowBuilder":
        self._steps.append((agent_name, task))
        return self

    def with_handoff_template(self, template: str) -> "WorkflowBuilder":
        self._handoff_template = template
        return self

    def build(self) -> Conversation:
        """Execute the workflow and return the conversation."""
        if not self._goal:
            raise ValueError("Goal is required")
        if not self._steps:
            raise ValueError("At least one step is required")

        return self.orchestrator.run_workflow(
            self._goal,
            self._steps,
            self._handoff_template
        )


# Common workflow patterns
def design_implement_review(
    orchestrator: Orchestrator,
    task: str
) -> Conversation:
    """
    Common pattern: Design -> Implement -> Review

    Good for feature development.
    """
    return (WorkflowBuilder(orchestrator)
        .goal(task)
        .then("architect", f"Design the architecture for: {task}")
        .then("implementer", "Implement the designed solution")
        .then("reviewer", "Review the implementation for quality and best practices")
        .build())


def research_design_implement(
    orchestrator: Orchestrator,
    task: str
) -> Conversation:
    """
    Pattern: Research -> Design -> Implement

    Good for exploring new domains.
    """
    return (WorkflowBuilder(orchestrator)
        .goal(task)
        .then("researcher", f"Research best practices and patterns for: {task}")
        .then("architect", "Design a solution based on the research")
        .then("implementer", "Implement the designed solution")
        .build())


def debug_fix_test(
    orchestrator: Orchestrator,
    issue: str
) -> Conversation:
    """
    Pattern: Troubleshoot -> Fix -> Test

    Good for bug fixing.
    """
    return (WorkflowBuilder(orchestrator)
        .goal(f"Fix: {issue}")
        .then("troubleshooter", f"Diagnose the issue: {issue}")
        .then("implementer", "Fix the identified problem")
        .then("tester", "Verify the fix works correctly")
        .build())
