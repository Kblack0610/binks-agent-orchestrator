"""
Factory Droid Runner - Wraps the `droid` CLI for headless agent execution

Supports:
- Headless execution via `droid exec`
- Tiered autonomy (low, medium, high)
- JSON output for parsing
- Model selection (Claude, GPT, Gemini backends)

Installation:
    curl -fsSL https://app.factory.ai/cli | sh

Environment:
    FACTORY_API_KEY=fk-...
"""
import json
import os
import shutil
import time
from typing import Optional, Dict, Any, List
from pathlib import Path

from .base import CLIRunner, RunnerResult


class FactoryRunner(CLIRunner):
    """
    Runner for Factory Droid CLI with headless execution support.

    Usage:
        runner = FactoryRunner()
        result = runner.run("Write a REST API for user management")
        print(result.content)

    With autonomy control:
        runner = FactoryRunner(autonomy="high")
        result = runner.run("Refactor this module", cwd="/path/to/project")
    """

    AUTONOMY_LEVELS = ["low", "medium", "high"]

    def __init__(
        self,
        model: Optional[str] = None,  # None = Factory default, or e.g. "claude-sonnet"
        autonomy: str = "medium",  # low, medium, high
        output_format: str = "json",  # text, json, stream-json, stream-jsonrpc
        working_dir: Optional[Path] = None,
        timeout: int = 300,
        debug: bool = False,
        reasoning_effort: Optional[str] = None,  # low, medium, high
        enabled_tools: Optional[List[str]] = None,  # Specific tools to enable
        skip_permissions: bool = False,  # Use --skip-permissions-unsafe
    ):
        super().__init__(
            name="factory",
            executable="droid",
            working_dir=working_dir,
            timeout=timeout,
            debug=debug
        )

        if autonomy not in self.AUTONOMY_LEVELS:
            raise ValueError(f"autonomy must be one of {self.AUTONOMY_LEVELS}")

        self.model = model
        self.autonomy = autonomy
        self.output_format = output_format
        self.reasoning_effort = reasoning_effort
        self.enabled_tools = enabled_tools
        self.skip_permissions = skip_permissions

    def is_available(self) -> bool:
        """Check if droid CLI is installed and API key is configured."""
        if not shutil.which("droid"):
            return False

        # Check for API key
        if not os.getenv("FACTORY_API_KEY"):
            return False

        return True

    def run(
        self,
        prompt: str,
        autonomy: Optional[str] = None,
        cwd: Optional[Path] = None,
        **kwargs
    ) -> RunnerResult:
        """
        Execute a prompt using Factory Droid CLI.

        Args:
            prompt: The task description
            autonomy: Override default autonomy level for this run
            cwd: Working directory for the droid (defaults to self.working_dir)
            **kwargs: Additional arguments

        Returns:
            RunnerResult with parsed response
        """
        start_time = time.time()

        # Build command
        cmd = ["droid", "exec"]

        # Set autonomy level
        level = autonomy or self.autonomy
        if level in self.AUTONOMY_LEVELS:
            cmd.extend(["--auto", level])

        # Output format
        cmd.extend(["-o", self.output_format])

        # Model selection
        if self.model:
            cmd.extend(["-m", self.model])

        # Reasoning effort
        if self.reasoning_effort:
            cmd.extend(["-r", self.reasoning_effort])

        # Working directory
        work_dir = cwd or self.working_dir
        if work_dir:
            cmd.extend(["--cwd", str(work_dir)])

        # Enabled tools
        if self.enabled_tools:
            cmd.extend(["--enabled-tools", ",".join(self.enabled_tools)])

        # Skip permissions (use with caution)
        if self.skip_permissions:
            cmd.append("--skip-permissions-unsafe")

        # Add the prompt
        cmd.append(prompt)

        # Execute
        stdout, stderr, returncode = self._execute_subprocess(cmd)
        execution_time = time.time() - start_time

        # Handle errors
        if returncode != 0:
            return RunnerResult(
                content="",
                backend="factory",
                model=self.model or "default",
                execution_time=execution_time,
                success=False,
                error=stderr or f"Exit code: {returncode}",
                raw_output=stdout
            )

        # Parse output based on format
        if self.output_format == "json":
            return self._parse_json_output(stdout, execution_time)
        else:
            return RunnerResult(
                content=stdout.strip(),
                backend="factory",
                model=self.model or "default",
                execution_time=execution_time,
                success=True,
                raw_output=stdout
            )

    def _parse_json_output(self, stdout: str, execution_time: float) -> RunnerResult:
        """Parse JSON output from droid exec."""
        try:
            data = json.loads(stdout)

            # Extract content from response structure
            content = ""
            if isinstance(data, dict):
                # Factory returns result in various fields
                content = data.get("result", data.get("content", data.get("message", "")))

                # Extract session ID if present
                self._session_id = data.get("session_id")

                # Get duration from response if available
                duration_ms = data.get("duration_ms", 0)
                if duration_ms and execution_time == 0:
                    execution_time = duration_ms / 1000.0

            elif isinstance(data, str):
                content = data

            return RunnerResult(
                content=content,
                session_id=self._session_id,
                backend="factory",
                model=self.model or "default",
                execution_time=execution_time,
                success=data.get("success", True) if isinstance(data, dict) else True,
                raw_output=stdout,
                metadata=data if isinstance(data, dict) else {}
            )

        except json.JSONDecodeError as e:
            # Fallback to plain text if JSON parsing fails
            return RunnerResult(
                content=stdout.strip(),
                backend="factory",
                model=self.model or "default",
                execution_time=execution_time,
                success=True,
                error=f"JSON parse warning: {e}",
                raw_output=stdout
            )

    def run_readonly(self, prompt: str, **kwargs) -> RunnerResult:
        """
        Execute in read-only mode (low autonomy).

        Useful for analysis tasks where no file modifications are needed.
        """
        return self.run(prompt, autonomy="low", **kwargs)

    def run_autonomous(self, prompt: str, **kwargs) -> RunnerResult:
        """
        Execute with high autonomy for fully automated tasks.

        Use with caution - this allows the agent to make significant changes.
        """
        return self.run(prompt, autonomy="high", **kwargs)


def check_factory_availability() -> dict:
    """
    Check Factory Droid availability and return diagnostic info.

    Returns:
        Dict with availability status and details
    """
    result = {
        "available": False,
        "cli_installed": False,
        "api_key_set": False,
        "version": None,
        "error": None
    }

    # Check CLI
    droid_path = shutil.which("droid")
    if droid_path:
        result["cli_installed"] = True
        result["cli_path"] = droid_path

        # Try to get version
        try:
            import subprocess
            version_result = subprocess.run(
                ["droid", "--version"],
                capture_output=True,
                text=True,
                timeout=5
            )
            if version_result.returncode == 0:
                result["version"] = version_result.stdout.strip()
        except Exception as e:
            result["error"] = f"Version check failed: {e}"
    else:
        result["error"] = "droid CLI not found. Install with: curl -fsSL https://app.factory.ai/cli | sh"

    # Check API key
    if os.getenv("FACTORY_API_KEY"):
        result["api_key_set"] = True
    else:
        result["error"] = result.get("error", "") + " FACTORY_API_KEY not set."

    result["available"] = result["cli_installed"] and result["api_key_set"]

    return result
