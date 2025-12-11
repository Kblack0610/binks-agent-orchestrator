"""
Gemini CLI Runner - Wraps Google's Gemini CLI

Supports multiple Gemini CLI implementations:
1. Google's official `gemini` CLI (if/when released)
2. Community tools like `aichat` with Gemini backend
3. Direct API calls as fallback

This is designed to be adaptable as Gemini CLI tools evolve.
"""
import json
import shutil
import os
from typing import Optional, List
from pathlib import Path

from .base import CLIRunner, RunnerResult


class GeminiRunner(CLIRunner):
    """
    Runner for Gemini CLI tools.

    Supports multiple backends that can talk to Gemini:
    - 'gemini': Official Google Gemini CLI (future)
    - 'aichat': aichat with Gemini backend
    - 'sgpt': shell-gpt with Gemini
    - 'api': Direct API call via Python

    Usage:
        runner = GeminiRunner(backend="api")  # Use API directly
        result = runner.run("Analyze this code for security issues")
    """

    SUPPORTED_BACKENDS = ["gemini", "cli", "aichat", "sgpt", "api"]

    # Map backend names to actual executable names
    BACKEND_EXECUTABLES = {
        "gemini": "gemini",
        "cli": "gemini",  # "cli" is an alias for "gemini"
        "aichat": "aichat",
        "sgpt": "sgpt",
        "api": "python"
    }

    def __init__(
        self,
        backend: str = "api",
        model: str = "gemini-1.5-pro",
        working_dir: Optional[Path] = None,
        timeout: int = 300,
        debug: bool = False,
        api_key: Optional[str] = None
    ):
        # Normalize backend name
        if backend == "cli":
            backend = "gemini"

        # Determine executable based on backend
        executable = self.BACKEND_EXECUTABLES.get(backend, backend)

        super().__init__(
            name=f"gemini-{backend}",
            executable=executable,
            working_dir=working_dir,
            timeout=timeout,
            debug=debug
        )

        self.backend = backend
        self.model = model
        self.api_key = api_key or os.getenv("GOOGLE_API_KEY") or os.getenv("GEMINI_API_KEY")

    def is_available(self) -> bool:
        """Check if the configured Gemini backend is available."""
        if self.backend == "api":
            # Check for API key and google-generativeai package
            if not self.api_key:
                return False
            try:
                import google.generativeai
                return True
            except ImportError:
                return False
        else:
            # Use the executable mapping to check the right binary
            executable = self.BACKEND_EXECUTABLES.get(self.backend, self.backend)
            return shutil.which(executable) is not None

    def run(self, prompt: str, **kwargs) -> RunnerResult:
        """
        Execute a prompt using the configured Gemini backend.

        Args:
            prompt: The prompt to send
            **kwargs: Additional backend-specific arguments

        Returns:
            RunnerResult with parsed response
        """
        if self.backend == "api":
            return self._run_api(prompt, **kwargs)
        elif self.backend == "aichat":
            return self._run_aichat(prompt, **kwargs)
        elif self.backend == "gemini":
            return self._run_gemini_cli(prompt, **kwargs)
        else:
            return RunnerResult(
                content="",
                backend=self.name,
                model=self.model,
                success=False,
                error=f"Unknown backend: {self.backend}"
            )

    def _run_api(self, prompt: str, **kwargs) -> RunnerResult:
        """Run using Google's generativeai Python package."""
        try:
            import google.generativeai as genai
            import time

            start_time = time.time()

            genai.configure(api_key=self.api_key)
            model = genai.GenerativeModel(self.model)

            response = model.generate_content(prompt)

            execution_time = time.time() - start_time

            # Extract text from response
            content = ""
            if response.text:
                content = response.text
            elif response.parts:
                content = "".join(part.text for part in response.parts if hasattr(part, 'text'))

            return RunnerResult(
                content=content,
                backend="gemini-api",
                model=self.model,
                execution_time=execution_time,
                success=True,
                metadata={
                    "finish_reason": str(response.candidates[0].finish_reason) if response.candidates else None,
                    "safety_ratings": [str(r) for r in response.candidates[0].safety_ratings] if response.candidates else []
                }
            )

        except ImportError:
            return RunnerResult(
                content="",
                backend="gemini-api",
                model=self.model,
                success=False,
                error="google-generativeai package not installed. Run: pip install google-generativeai"
            )
        except Exception as e:
            return RunnerResult(
                content="",
                backend="gemini-api",
                model=self.model,
                success=False,
                error=str(e)
            )

    def _run_aichat(self, prompt: str, **kwargs) -> RunnerResult:
        """Run using aichat CLI with Gemini backend."""
        cmd = [
            "aichat",
            "--model", f"gemini:{self.model}",
            prompt
        ]

        stdout, stderr, returncode = self._execute_subprocess(cmd)

        if returncode != 0:
            return RunnerResult(
                content="",
                backend="gemini-aichat",
                model=self.model,
                success=False,
                error=stderr or f"Exit code: {returncode}"
            )

        return RunnerResult(
            content=stdout.strip(),
            backend="gemini-aichat",
            model=self.model,
            success=True,
            raw_output=stdout
        )

    def _run_gemini_cli(self, prompt: str, **kwargs) -> RunnerResult:
        """Run using official Google Gemini CLI.

        The Gemini CLI (https://github.com/google-gemini/gemini-cli) uses:
        - `gemini "prompt"` for one-shot queries
        - `gemini -m model "prompt"` for specific model
        """
        import time

        start_time = time.time()

        # Build command - Gemini CLI takes prompt as positional argument
        cmd = ["gemini"]

        # Add model if specified (not default)
        if self.model and self.model != "gemini-1.5-pro":
            cmd.extend(["-m", self.model])

        # Add the prompt as positional argument
        cmd.append(prompt)

        stdout, stderr, returncode = self._execute_subprocess(cmd)
        execution_time = time.time() - start_time

        if returncode != 0:
            return RunnerResult(
                content="",
                backend="gemini-cli",
                model=self.model,
                execution_time=execution_time,
                success=False,
                error=stderr or f"Exit code: {returncode}"
            )

        return RunnerResult(
            content=stdout.strip(),
            backend="gemini-cli",
            model=self.model,
            execution_time=execution_time,
            success=True,
            raw_output=stdout
        )


def check_gemini_availability() -> dict:
    """
    Check which Gemini backends are available.

    Returns:
        Dict with backend names and their availability status
    """
    results = {}

    # Check for API
    api_key = os.getenv("GOOGLE_API_KEY") or os.getenv("GEMINI_API_KEY")
    try:
        import google.generativeai
        results["api"] = bool(api_key)
    except ImportError:
        results["api"] = False

    # Check for CLI tools
    for tool in ["gemini", "aichat", "sgpt"]:
        results[tool] = shutil.which(tool) is not None

    return results
