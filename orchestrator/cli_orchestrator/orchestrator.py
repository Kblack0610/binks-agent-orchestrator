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
    from .agent import Agent, AgentResponse, AgentRole as NewAgentRole, create_agent
    from .memory_bank import MemoryBank
except ImportError:
    from runners.base import CLIRunner, RunnerResult
    from profiles import AgentProfile, ProfileRegistry, AgentRole
    from agent import Agent, AgentResponse, AgentRole as NewAgentRole, create_agent
    from memory_bank import MemoryBank


class TaskStatus(Enum):
    """Status tracking for MoA workflows."""
    PLANNING = "planning"
    CODING = "coding"
    REVIEWING = "reviewing"
    VERIFYING = "verifying"
    COMPLETED = "completed"
    FAILED = "failed"
    PAUSED = "paused"


@dataclass
class ConvergenceCriteria:
    """Criteria for determining when MoA workflow should stop."""
    max_iterations: int = 5
    require_critic_pass: bool = True
    require_tests_pass: bool = False
    context_char_limit: int = 50000  # Trigger compaction (~12k tokens)

    def should_stop(
        self,
        iteration: int,
        critic_verdict: str,
        test_result: bool = True
    ) -> tuple[bool, str]:
        """
        Check if workflow should stop.

        Returns:
            (should_stop, reason) tuple
        """
        if iteration >= self.max_iterations:
            return True, "max_iterations_reached"
        if self.require_critic_pass and critic_verdict == "PASS":
            if not self.require_tests_pass or test_result:
                return True, "success"
        return False, "continue"


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
    Supports both legacy ProfileRegistry and new agnostic Agent architecture.
    """

    def __init__(
        self,
        registry: ProfileRegistry = None,
        working_dir: Optional[Path] = None,
        max_turns: int = 20,
        debug: bool = False,
        memory_bank: Optional[MemoryBank] = None
    ):
        self.registry = registry
        self.working_dir = working_dir or Path.cwd()
        self.max_turns = max_turns
        self.debug = debug
        self._conversations: Dict[str, Conversation] = {}

        # New agnostic architecture components
        self.memory_bank = memory_bank or MemoryBank(self.working_dir / ".orchestrator")
        self._current_status: TaskStatus = TaskStatus.PLANNING

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

    def run_moa_workflow(
        self,
        goal: str,
        architect: Agent,
        executor: Agent,
        critic: Agent = None,
        convergence: ConvergenceCriteria = None
    ) -> Conversation:
        """
        Run a Mixture of Agents (MoA) workflow with convergence checking.

        This is the new agnostic architecture - agents are passed in, NOT
        hardcoded to specific backends. Any Agent can play any role.

        Args:
            goal: The task/feature to implement
            architect: Agent configured for planning (any backend)
            executor: Agent configured for implementation (any backend)
            critic: Agent for review (defaults to architect if not provided)
            convergence: Convergence criteria for stopping

        Returns:
            Completed Conversation with all turns

        Example:
            from agent import create_agent, AgentRole
            from runners import ClaudeRunner, GeminiRunner

            architect = create_agent("arch", GeminiRunner(), AgentRole.ARCHITECT)
            executor = create_agent("impl", ClaudeRunner(), AgentRole.EXECUTOR)

            result = orchestrator.run_moa_workflow(
                goal="Build REST API",
                architect=architect,
                executor=executor
            )
        """
        convergence = convergence or ConvergenceCriteria()
        critic = critic or architect  # Reuse architect for critique if not provided

        # Initialize
        conversation = self.create_conversation(goal)
        self.memory_bank.initialize(goal)
        iteration = 0

        # Always show workflow info
        print(f"\n🚀 Starting MoA workflow...")
        print(f"   Architect: {architect.runner.name}")
        print(f"   Executor:  {executor.runner.name}")
        print(f"   Critic:    {critic.runner.name}")

        while True:
            iteration += 1
            context = self.memory_bank.read_context()

            # Check context size for compaction
            if len(context) > convergence.context_char_limit:
                if self.debug:
                    print(f"[MoA] Context too large ({len(context)} chars), compacting...")
                self.memory_bank.compact(architect.runner)
                context = self.memory_bank.read_context()

            # Phase 1: PLANNING
            self._current_status = TaskStatus.PLANNING
            print(f"\n⏳ [{iteration}] Architect planning...", end="", flush=True)

            start_time = time.time()
            plan_response = architect.invoke(
                f"Design solution for: {goal}",
                context=context
            )
            plan_time = time.time() - start_time
            summary = plan_response.content[:80].replace('\n', ' ').strip()
            print(f" ✓ ({plan_time:.1f}s)")
            print(f"   → {summary}...")

            plan_turn = ConversationTurn(
                agent_name=architect.name,
                role=AgentRole.ARCHITECT,
                prompt=f"Design solution for: {goal}",
                response=plan_response.content,
                execution_time=plan_time,
                backend=architect.runner.name,
                success=plan_response.success
            )
            conversation.add_turn(plan_turn)

            # Phase 2: CODING
            self._current_status = TaskStatus.CODING
            print(f"⏳ [{iteration}] Executor implementing...", end="", flush=True)

            start_time = time.time()
            impl_response = executor.invoke(
                "Implement this design",
                context=plan_response.content
            )
            impl_time = time.time() - start_time
            summary = impl_response.content[:80].replace('\n', ' ').strip()
            print(f" ✓ ({impl_time:.1f}s)")
            print(f"   → {summary}...")

            impl_turn = ConversationTurn(
                agent_name=executor.name,
                role=AgentRole.IMPLEMENTER,
                prompt="Implement this design",
                response=impl_response.content,
                execution_time=impl_time,
                backend=executor.runner.name,
                success=impl_response.success
            )
            conversation.add_turn(impl_turn)

            # Phase 3: REVIEWING
            self._current_status = TaskStatus.REVIEWING
            print(f"⏳ [{iteration}] Critic reviewing...", end="", flush=True)

            start_time = time.time()
            review_response = critic.invoke(
                """Review this implementation briefly.
Is it correct and complete for the task?

You MUST end your response with exactly one of:
VERDICT: PASS
or
VERDICT: FAIL""",
                context=impl_response.content
            )
            review_time = time.time() - start_time
            verdict = review_response.verdict or "NO VERDICT"
            summary = review_response.content[:80].replace('\n', ' ').strip()
            print(f" ✓ ({review_time:.1f}s) → {verdict}")
            print(f"   → {summary}...")

            review_turn = ConversationTurn(
                agent_name=critic.name,
                role=AgentRole.REVIEWER,
                prompt="Review implementation",
                response=review_response.content,
                execution_time=review_time,
                backend=critic.runner.name,
                success=review_response.success
            )
            conversation.add_turn(review_turn)

            # Update memory bank with progress
            self.memory_bank.update_active_context(f"""## Current State
- Status: ITERATION {iteration}
- Phase: REVIEWING (just completed)
- Verdict: {review_response.verdict or 'PENDING'}

## Latest Plan Summary
{plan_response.content[:500]}...

## Latest Implementation Summary
{impl_response.content[:500]}...

## Latest Review
{review_response.content[:500]}...

## Next Steps
{'Fix issues and iterate' if review_response.verdict != 'PASS' else 'Task complete!'}
""")

            # Phase 4: Check convergence
            should_stop, reason = convergence.should_stop(
                iteration,
                review_response.verdict or ""
            )

            if self.debug:
                print(f"[MoA] Verdict: {review_response.verdict}, Reason: {reason}")

            if should_stop:
                if reason == "success":
                    self._current_status = TaskStatus.COMPLETED
                    conversation.status = "completed"
                    print(f"\n✅ SUCCESS after {iteration} iteration(s)!")
                else:
                    self._current_status = TaskStatus.FAILED
                    conversation.status = "failed"
                    print(f"\n⚠️  STOPPED: {reason}")
                break

            # Phase 5: FIX (if not converged)
            print(f"⏳ [{iteration}] Executor fixing...", end="", flush=True)

            start_time = time.time()
            fix_response = executor.invoke(
                f"Fix based on this feedback: {review_response.content}",
                context=impl_response.content
            )
            fix_time = time.time() - start_time
            summary = fix_response.content[:80].replace('\n', ' ').strip()
            print(f" ✓ ({fix_time:.1f}s)")
            print(f"   → {summary}...")

            fix_turn = ConversationTurn(
                agent_name=executor.name,
                role=AgentRole.IMPLEMENTER,
                prompt="Fix based on feedback",
                response=fix_response.content,
                execution_time=fix_time,
                backend=executor.runner.name,
                success=fix_response.success
            )
            conversation.add_turn(fix_turn)

        # Final summary
        conversation.metadata["iterations"] = iteration
        conversation.metadata["final_status"] = self._current_status.value

        return conversation

    def get_status(self) -> Dict[str, Any]:
        """Get current orchestrator status for CLI."""
        return {
            "status": self._current_status.value,
            "active_conversations": len(self._conversations),
            "memory_bank_initialized": self.memory_bank.active_context.exists(),
            "working_dir": str(self.working_dir)
        }


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
