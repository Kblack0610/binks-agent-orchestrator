"""
Headless CLI Tools for Agno - Wraps CLI tools with session persistence.

The Agno Agent handles its own state via SqliteAgentStorage.
This toolkit only tracks CLI-specific session IDs.

Architecture:
    - Agno Native (SqliteAgentStorage) = Manager's brain
    - CLI Sessions (.cli_sessions.json) = Bridge to CLI tools
    - Memory Bank (.orchestrator/*.md) = Project context (human-readable, Git-tracked)
"""
import subprocess
import json
import shutil
from pathlib import Path
from typing import Optional, Dict, Any
from datetime import datetime

# Try Agno import, fall back to standalone usage
try:
    from agno.tools import Toolkit
    AGNO_AVAILABLE = True
except ImportError:
    # Standalone usage without Agno
    class Toolkit:
        def __init__(self, name: str):
            self.name = name
        def register(self, func):
            pass
    AGNO_AVAILABLE = False


class HeadlessCliTools(Toolkit):
    """
    CLI Tools that remember their sessions across invocations.

    Agno handles the Manager's memory. This handles the CLI's memory.

    Usage with Agno:
        from agno.agent import Agent
        from tools.headless_tools import HeadlessCliTools

        agent = Agent(
            name="Manager",
            tools=[HeadlessCliTools()],
            ...
        )

    Standalone usage:
        tools = HeadlessCliTools()
        result = tools.run_claude("Design a REST API", role="architect")
    """

    def __init__(
        self,
        session_file: str = ".cli_sessions.json",
        memory_dir: str = ".orchestrator"
    ):
        super().__init__(name="headless_cli")
        self.session_file = Path(session_file)
        self.memory_dir = Path(memory_dir)

        # Register tools for Agno
        self.register(self.run_claude)
        self.register(self.run_gemini)
        self.register(self.run_kilocode)
        self.register(self.read_memory_bank)
        self.register(self.update_active_context)
        self.register(self.initialize_project)

    # =========================================================================
    # Session Management
    # =========================================================================

    def _get_session(self, task_name: str) -> Optional[str]:
        """Recall CLI session ID for a specific role."""
        if self.session_file.exists():
            with open(self.session_file) as f:
                return json.load(f).get(task_name)
        return None

    def _save_session(self, task_name: str, session_id: str) -> None:
        """Persist CLI session ID."""
        data = {}
        if self.session_file.exists():
            with open(self.session_file) as f:
                data = json.load(f)
        data[task_name] = session_id
        with open(self.session_file, 'w') as f:
            json.dump(data, f, indent=2)

    def clear_sessions(self) -> None:
        """Clear all saved sessions (start fresh)."""
        if self.session_file.exists():
            self.session_file.unlink()

    # =========================================================================
    # CLI Tools
    # =========================================================================

    def run_claude(self, task: str, role: str = "architect") -> str:
        """
        Run Claude CLI with automatic session resumption.

        Args:
            task: The task/prompt to send
            role: Role name for session tracking and SuperClaude profile
                  (architect, executor, analyst, researcher, etc.)

        Returns:
            Claude's response text
        """
        if not shutil.which("claude"):
            return "Error: Claude CLI not installed. Run: npm install -g @anthropic-ai/claude-code"

        session_id = self._get_session(f"claude_{role}")

        # Build command with SuperClaude profile
        cmd = ["claude", "-p", f"/sc:{role} {task}", "--output-format", "json"]
        if session_id:
            cmd.extend(["--resume", session_id])

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=600,
                cwd=str(Path.cwd())
            )

            # Try to parse JSON response
            try:
                response = json.loads(result.stdout)

                # Save session for next turn
                if 'session_id' in response:
                    self._save_session(f"claude_{role}", response['session_id'])

                # Extract content from various response formats
                content = (
                    response.get('result') or
                    response.get('content') or
                    response.get('message', {}).get('content') or
                    str(response)
                )
                return content

            except json.JSONDecodeError:
                # Return raw output if not JSON
                return result.stdout.strip() or result.stderr.strip()

        except subprocess.TimeoutExpired:
            return "Error: Claude CLI timed out after 10 minutes"
        except Exception as e:
            return f"Error running Claude: {e}"

    def run_gemini(self, task: str, role: str = "architect") -> str:
        """
        Run Gemini API/CLI for planning and review tasks.

        Args:
            task: The task/prompt to send
            role: Role context (for prompt formatting)

        Returns:
            Gemini's response text
        """
        # Try google-generativeai API first
        try:
            import google.generativeai as genai
            import os

            api_key = os.environ.get("GOOGLE_API_KEY")
            if not api_key:
                return "Error: GOOGLE_API_KEY not set"

            genai.configure(api_key=api_key)
            model = genai.GenerativeModel("gemini-1.5-pro")

            # Add role context to prompt
            full_prompt = f"You are acting as a {role}.\n\nTask: {task}"
            response = model.generate_content(full_prompt)

            return response.text

        except ImportError:
            # Fall back to CLI if available
            if shutil.which("gemini"):
                try:
                    result = subprocess.run(
                        ["gemini", task],
                        capture_output=True,
                        text=True,
                        timeout=300
                    )
                    return result.stdout.strip()
                except Exception as e:
                    return f"Error running Gemini CLI: {e}"

            return "Error: Neither google-generativeai package nor Gemini CLI available"

        except Exception as e:
            return f"Error running Gemini: {e}"

    def run_kilocode(self, task: str, auto: bool = True) -> str:
        """
        Run KiloCode in autonomous mode.

        Args:
            task: The task/prompt to send
            auto: Whether to run in autonomous mode (default: True)

        Returns:
            KiloCode's response or status
        """
        if not shutil.which("kilocode"):
            return "Error: KiloCode CLI not installed"

        cmd = ["kilocode"]
        if auto:
            cmd.append("--auto")
        cmd.append(task)

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=1800  # 30 minutes for longer tasks
            )
            return result.stdout.strip() or "KiloCode task completed."
        except subprocess.TimeoutExpired:
            return "Error: KiloCode timed out after 30 minutes"
        except Exception as e:
            return f"KiloCode error: {e}"

    # =========================================================================
    # Memory Bank (Project Context)
    # =========================================================================

    def initialize_project(self, goal: str, project_info: str = "") -> str:
        """
        Initialize the memory bank for a new project/task.

        Args:
            goal: The high-level goal for this project
            project_info: Additional project context

        Returns:
            Confirmation message
        """
        self.memory_dir.mkdir(exist_ok=True)

        # Product Context - The "Why"
        product_context = self.memory_dir / "productContext.md"
        product_context.write_text(f"""# Product Context

## Goal
{goal}

## Project Info
{project_info}

## Success Criteria
- [ ] Define during planning phase
""")

        # Active Context - The "What"
        active_context = self.memory_dir / "activeContext.md"
        active_context.write_text(f"""# Active Context
_Last updated: {datetime.now().isoformat()}_

## Current State
- Status: INITIALIZED
- Phase: PLANNING

## What We're Working On
(To be updated by agents)

## Recent Progress
(Empty - just started)

## Next Steps
1. Generate implementation plan
""")

        # System Patterns - Architectural decisions
        system_patterns = self.memory_dir / "systemPatterns.md"
        system_patterns.write_text("""# System Patterns

## Architectural Decisions
(To be populated during design phase)

## Code Patterns
(To be populated during implementation)

## Lessons Learned
(Updated after each iteration)
""")

        return f"Project initialized in {self.memory_dir}/"

    def read_memory_bank(self) -> str:
        """
        Read the project's memory bank (all context files).

        This is the source of truth for project state.

        Returns:
            Combined content of all memory bank files
        """
        parts = []

        # Read each context file if it exists
        for filename in ["productContext.md", "activeContext.md", "systemPatterns.md"]:
            filepath = self.memory_dir / filename
            if filepath.exists():
                parts.append(f"=== {filename.upper()} ===\n{filepath.read_text()}")

        if not parts:
            return "No memory bank found. Call initialize_project() first."

        return "\n\n".join(parts)

    def update_active_context(self, content: str) -> str:
        """
        Update the project's active context.

        This should be called after every significant step to track progress.

        Args:
            content: New content for activeContext.md

        Returns:
            Confirmation message
        """
        self.memory_dir.mkdir(exist_ok=True)

        header = f"# Active Context\n_Last updated: {datetime.now().isoformat()}_\n\n"
        (self.memory_dir / "activeContext.md").write_text(header + content)

        return "Active context updated."

    def append_pattern(self, pattern_name: str, pattern_content: str) -> str:
        """
        Add a new pattern to system patterns.

        Args:
            pattern_name: Name/title of the pattern
            pattern_content: Description of the pattern

        Returns:
            Confirmation message
        """
        patterns_file = self.memory_dir / "systemPatterns.md"

        current = ""
        if patterns_file.exists():
            current = patterns_file.read_text()

        new_content = f"{current}\n\n## {pattern_name}\n{pattern_content}"
        patterns_file.write_text(new_content)

        return f"Pattern '{pattern_name}' added."

    # =========================================================================
    # Utility Methods
    # =========================================================================

    def check_available_tools(self) -> Dict[str, bool]:
        """Check which CLI tools are available."""
        return {
            "claude": shutil.which("claude") is not None,
            "gemini_api": self._check_gemini_api(),
            "gemini_cli": shutil.which("gemini") is not None,
            "kilocode": shutil.which("kilocode") is not None,
        }

    def _check_gemini_api(self) -> bool:
        """Check if Gemini API is available."""
        try:
            import google.generativeai
            import os
            return bool(os.environ.get("GOOGLE_API_KEY"))
        except ImportError:
            return False


# Standalone usage example
if __name__ == "__main__":
    tools = HeadlessCliTools()

    print("Available tools:")
    for tool, available in tools.check_available_tools().items():
        status = "available" if available else "not available"
        print(f"  {tool}: {status}")

    print("\nInitializing test project...")
    print(tools.initialize_project("Test the headless tools", "CLI Orchestrator"))

    print("\nReading memory bank:")
    print(tools.read_memory_bank())
