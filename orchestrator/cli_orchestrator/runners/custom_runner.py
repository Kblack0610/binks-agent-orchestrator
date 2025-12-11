"""
Custom Runner - Run arbitrary Python modules or shell scripts as agents

This allows you to integrate any custom code into the orchestration:
- Python functions/modules
- Shell scripts
- Local Ollama models
- KiloCode MCP integration
- Any executable that takes input and produces output
"""
import importlib.util
import subprocess
import shutil
from typing import Optional, Callable, Any
from pathlib import Path

from .base import CLIRunner, RunnerResult


class CustomRunner(CLIRunner):
    """
    Runner for custom Python modules, scripts, or executables.

    Supports three modes:
    1. Python callable: Pass a function directly
    2. Python module: Load and call a function from a .py file
    3. Shell executable: Run any CLI tool

    Usage:
        # Mode 1: Python callable
        def my_agent(prompt):
            return f"Processed: {prompt}"
        runner = CustomRunner.from_callable(my_agent, name="my-agent")

        # Mode 2: Python module
        runner = CustomRunner.from_module("./agents/reviewer.py", "review")

        # Mode 3: Shell executable
        runner = CustomRunner.from_executable("./scripts/analyze.sh")
    """

    def __init__(
        self,
        name: str,
        executable: Optional[str] = None,
        callable_fn: Optional[Callable[[str], str]] = None,
        module_path: Optional[Path] = None,
        function_name: Optional[str] = None,
        working_dir: Optional[Path] = None,
        timeout: int = 300,
        debug: bool = False
    ):
        super().__init__(
            name=name,
            executable=executable or "python",
            working_dir=working_dir,
            timeout=timeout,
            debug=debug
        )
        self.callable_fn = callable_fn
        self.module_path = module_path
        self.function_name = function_name or "run"

        # Load module function if specified
        if module_path and not callable_fn:
            self.callable_fn = self._load_module_function()

    def _load_module_function(self) -> Optional[Callable]:
        """Dynamically load a function from a Python module."""
        try:
            spec = importlib.util.spec_from_file_location(
                "custom_module",
                self.module_path
            )
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)

            fn = getattr(module, self.function_name, None)
            if fn and callable(fn):
                return fn
            else:
                if self.debug:
                    print(f"Function '{self.function_name}' not found in {self.module_path}")
                return None

        except Exception as e:
            if self.debug:
                print(f"Error loading module: {e}")
            return None

    def is_available(self) -> bool:
        """Check if this custom runner is available."""
        if self.callable_fn:
            return True
        if self.executable:
            return shutil.which(self.executable) is not None or Path(self.executable).exists()
        return False

    def run(self, prompt: str, **kwargs) -> RunnerResult:
        """
        Execute the custom runner.

        Args:
            prompt: Input to pass to the runner
            **kwargs: Additional arguments

        Returns:
            RunnerResult
        """
        if self.callable_fn:
            return self._run_callable(prompt, **kwargs)
        else:
            return self._run_executable(prompt, **kwargs)

    def _run_callable(self, prompt: str, **kwargs) -> RunnerResult:
        """Run a Python callable."""
        import time

        start_time = time.time()

        try:
            result = self.callable_fn(prompt, **kwargs)
            execution_time = time.time() - start_time

            # Handle different return types
            if isinstance(result, RunnerResult):
                return result
            elif isinstance(result, dict):
                return RunnerResult(
                    content=result.get("content", str(result)),
                    backend=self.name,
                    execution_time=execution_time,
                    success=result.get("success", True),
                    metadata=result
                )
            else:
                return RunnerResult(
                    content=str(result),
                    backend=self.name,
                    execution_time=execution_time,
                    success=True
                )

        except Exception as e:
            return RunnerResult(
                content="",
                backend=self.name,
                success=False,
                error=str(e)
            )

    def _run_executable(self, prompt: str, **kwargs) -> RunnerResult:
        """Run a shell executable."""
        import time

        # Build command - pass prompt as argument or via stdin
        cmd = [self.executable, prompt]

        start_time = time.time()
        stdout, stderr, returncode = self._execute_subprocess(cmd)
        execution_time = time.time() - start_time

        if returncode != 0:
            return RunnerResult(
                content="",
                backend=self.name,
                execution_time=execution_time,
                success=False,
                error=stderr or f"Exit code: {returncode}",
                raw_output=stdout
            )

        return RunnerResult(
            content=stdout.strip(),
            backend=self.name,
            execution_time=execution_time,
            success=True,
            raw_output=stdout
        )

    @classmethod
    def from_callable(
        cls,
        fn: Callable[[str], Any],
        name: str,
        **kwargs
    ) -> "CustomRunner":
        """Create a CustomRunner from a Python callable."""
        return cls(name=name, callable_fn=fn, **kwargs)

    @classmethod
    def from_module(
        cls,
        module_path: str,
        function_name: str = "run",
        name: Optional[str] = None,
        **kwargs
    ) -> "CustomRunner":
        """Create a CustomRunner from a Python module file."""
        path = Path(module_path)
        runner_name = name or path.stem
        return cls(
            name=runner_name,
            module_path=path,
            function_name=function_name,
            **kwargs
        )

    @classmethod
    def from_executable(
        cls,
        executable: str,
        name: Optional[str] = None,
        **kwargs
    ) -> "CustomRunner":
        """Create a CustomRunner from a shell executable."""
        runner_name = name or Path(executable).stem
        return cls(name=runner_name, executable=executable, **kwargs)


class OllamaRunner(CustomRunner):
    """
    Specialized runner for Ollama models (local or remote).

    Supports two modes:
    - Local: Uses `ollama` CLI (requires ollama installed)
    - Remote: Uses HTTP API (for remote Ollama servers)

    Usage:
        # Local (default)
        runner = OllamaRunner(model="llama3.1:8b")

        # Remote (e.g., ollama-home on 192.168.1.4)
        runner = OllamaRunner(model="llama3.1:8b", host="http://192.168.1.4:11434")
    """

    def __init__(
        self,
        model: str = "llama3.1:8b",
        host: str = "http://localhost:11434",
        name: Optional[str] = None,
        use_api: bool = None,  # Auto-detect if None
        **kwargs
    ):
        super().__init__(
            name=name or f"ollama-{model}",
            executable="ollama",
            **kwargs
        )
        self.model = model
        self.host = host.rstrip('/')

        # Auto-detect: use API for remote hosts, CLI for localhost
        if use_api is None:
            self.use_api = "localhost" not in host and "127.0.0.1" not in host
        else:
            self.use_api = use_api

    def is_available(self) -> bool:
        """Check if Ollama is running."""
        import urllib.request
        try:
            urllib.request.urlopen(f"{self.host}/api/tags", timeout=3)
            return True
        except:
            return False

    def run(self, prompt: str, **kwargs) -> RunnerResult:
        """Run a prompt through Ollama (CLI or API)."""
        if self.use_api:
            return self._run_api(prompt, **kwargs)
        else:
            return self._run_cli(prompt, **kwargs)

    def _run_cli(self, prompt: str, **kwargs) -> RunnerResult:
        """Run via ollama CLI (local only)."""
        import time

        cmd = ["ollama", "run", self.model, prompt]

        start_time = time.time()
        stdout, stderr, returncode = self._execute_subprocess(cmd)
        execution_time = time.time() - start_time

        if returncode != 0:
            return RunnerResult(
                content="",
                backend=self.name,
                model=self.model,
                execution_time=execution_time,
                success=False,
                error=stderr
            )

        return RunnerResult(
            content=stdout.strip(),
            backend=self.name,
            model=self.model,
            execution_time=execution_time,
            success=True
        )

    def _run_api(self, prompt: str, **kwargs) -> RunnerResult:
        """Run via Ollama HTTP API (works for remote servers)."""
        import time
        import json
        import urllib.request
        import urllib.error

        start_time = time.time()

        try:
            data = json.dumps({
                "model": self.model,
                "prompt": prompt,
                "stream": False
            }).encode('utf-8')

            req = urllib.request.Request(
                f"{self.host}/api/generate",
                data=data,
                headers={"Content-Type": "application/json"}
            )

            with urllib.request.urlopen(req, timeout=self.timeout) as response:
                result = json.loads(response.read().decode('utf-8'))
                execution_time = time.time() - start_time

                return RunnerResult(
                    content=result.get("response", "").strip(),
                    backend=self.name,
                    model=self.model,
                    execution_time=execution_time,
                    success=True
                )

        except urllib.error.URLError as e:
            return RunnerResult(
                content="",
                backend=self.name,
                model=self.model,
                execution_time=time.time() - start_time,
                success=False,
                error=f"Connection error: {e.reason}"
            )
        except Exception as e:
            return RunnerResult(
                content="",
                backend=self.name,
                model=self.model,
                execution_time=time.time() - start_time,
                success=False,
                error=str(e)
            )


# Convenience constants for common Ollama hosts
OLLAMA_LOCAL = "http://localhost:11434"
OLLAMA_HOME = "http://192.168.1.4:11434"
