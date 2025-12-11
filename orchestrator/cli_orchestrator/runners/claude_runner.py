"""
Claude CLI Runner - Wraps the `claude` CLI with SuperClaude support

Supports:
- Headless mode (-p/--print)
- JSON output for parsing
- Session continuity (--resume)
- SuperClaude slash commands (/sc:design, /sc:implement, etc.)
"""
import json
import shutil
from typing import Optional, Dict, Any
from pathlib import Path

from .base import CLIRunner, RunnerResult


class ClaudeRunner(CLIRunner):
    """
    Runner for Claude Code CLI with SuperClaude integration.

    Usage:
        runner = ClaudeRunner()
        result = runner.run("/sc:design Build a REST API for user management")
        print(result.content)
    """

    def __init__(
        self,
        working_dir: Optional[Path] = None,
        model: str = "sonnet",  # sonnet, opus, haiku
        output_format: str = "json",  # text, json, stream-json
        timeout: int = 600,
        debug: bool = False
    ):
        super().__init__(
            name="claude",
            executable="claude",
            working_dir=working_dir,
            timeout=timeout,
            debug=debug
        )
        self.model = model
        self.output_format = output_format

    def is_available(self) -> bool:
        """Check if claude CLI is installed and accessible."""
        return shutil.which("claude") is not None

    def run(
        self,
        prompt: str,
        profile: Optional[str] = None,
        resume_session: bool = False,
        system_prompt: Optional[str] = None,
        **kwargs
    ) -> RunnerResult:
        """
        Execute a prompt using Claude CLI.

        Args:
            prompt: The prompt to send (can include /sc:* commands)
            profile: Optional SuperClaude profile to prepend (e.g., "design", "implement")
            resume_session: If True, resume from previous session_id
            system_prompt: Optional system prompt override
            **kwargs: Additional arguments

        Returns:
            RunnerResult with parsed response
        """
        # Build the full prompt with optional profile
        full_prompt = prompt
        if profile and not prompt.startswith("/sc:"):
            full_prompt = f"/sc:{profile} {prompt}"

        # Build command
        cmd = [
            self.executable,
            "-p",  # Print mode (headless)
            "--output-format", self.output_format,
        ]

        # Resume session if requested and we have a session ID
        if resume_session and self._session_id:
            cmd.extend(["--resume", self._session_id])

        # Add the prompt
        cmd.append(full_prompt)

        # Execute
        stdout, stderr, returncode = self._execute_subprocess(cmd)

        # Parse result
        if returncode != 0:
            return RunnerResult(
                content="",
                backend="claude",
                model=self.model,
                success=False,
                error=stderr or f"Exit code: {returncode}",
                raw_output=stdout
            )

        return self._parse_output(stdout, stderr)

    def _parse_output(self, stdout: str, stderr: str) -> RunnerResult:
        """Parse Claude CLI output based on format."""

        if self.output_format == "json":
            return self._parse_json_output(stdout)
        else:
            return RunnerResult(
                content=stdout.strip(),
                backend="claude",
                model=self.model,
                success=True,
                raw_output=stdout
            )

    def _parse_json_output(self, stdout: str) -> RunnerResult:
        """Parse JSON output from Claude CLI."""
        try:
            data = json.loads(stdout)

            # Extract session ID for continuity
            self._session_id = data.get("session_id")

            # Handle different response structures
            content = ""
            if "result" in data:
                content = data["result"]
            elif "content" in data:
                content = data["content"]
            elif "message" in data:
                msg = data["message"]
                if isinstance(msg, dict):
                    content = msg.get("content", "")
                else:
                    content = str(msg)

            return RunnerResult(
                content=content,
                session_id=self._session_id,
                backend="claude",
                model=self.model,
                success=True,
                raw_output=stdout,
                metadata=data
            )

        except json.JSONDecodeError as e:
            # Fallback: treat as plain text
            return RunnerResult(
                content=stdout.strip(),
                backend="claude",
                model=self.model,
                success=True,
                error=f"JSON parse warning: {e}",
                raw_output=stdout
            )

    def run_with_profile(
        self,
        prompt: str,
        profile: str,
        **kwargs
    ) -> RunnerResult:
        """
        Convenience method to run with a specific SuperClaude profile.

        Args:
            prompt: The task description
            profile: SuperClaude profile (design, implement, analyze, etc.)

        Returns:
            RunnerResult
        """
        return self.run(f"/sc:{profile} {prompt}", **kwargs)

    # Convenience methods for common SuperClaude profiles
    def design(self, task: str, **kwargs) -> RunnerResult:
        """Run with /sc:design profile."""
        return self.run_with_profile(task, "design", **kwargs)

    def implement(self, task: str, **kwargs) -> RunnerResult:
        """Run with /sc:implement profile."""
        return self.run_with_profile(task, "implement", **kwargs)

    def analyze(self, task: str, **kwargs) -> RunnerResult:
        """Run with /sc:analyze profile."""
        return self.run_with_profile(task, "analyze", **kwargs)

    def troubleshoot(self, task: str, **kwargs) -> RunnerResult:
        """Run with /sc:troubleshoot profile."""
        return self.run_with_profile(task, "troubleshoot", **kwargs)

    def research(self, task: str, **kwargs) -> RunnerResult:
        """Run with /sc:research profile."""
        return self.run_with_profile(task, "research", **kwargs)
