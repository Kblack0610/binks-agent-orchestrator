"""
Base CLI Runner - Abstract interface for AI CLI backends
"""
import subprocess
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Optional, Dict, Any
from pathlib import Path


@dataclass
class RunnerResult:
    """Result from a CLI runner execution."""
    content: str
    session_id: Optional[str] = None
    model: str = ""
    backend: str = ""
    execution_time: float = 0.0
    tokens_used: Optional[int] = None
    success: bool = True
    error: Optional[str] = None
    raw_output: Optional[str] = None
    metadata: Dict[str, Any] = field(default_factory=dict)

    def __str__(self) -> str:
        return self.content


class CLIRunner(ABC):
    """
    Abstract base class for CLI-based AI backends.

    Subclasses implement specific CLI tools (Claude, Gemini, etc.)
    """

    def __init__(
        self,
        name: str,
        executable: str,
        working_dir: Optional[Path] = None,
        timeout: int = 300,
        debug: bool = False
    ):
        self.name = name
        self.executable = executable
        self.working_dir = working_dir or Path.cwd()
        self.timeout = timeout
        self.debug = debug
        self._session_id: Optional[str] = None

    @abstractmethod
    def run(self, prompt: str, **kwargs) -> RunnerResult:
        """Execute a prompt and return the result."""
        pass

    @abstractmethod
    def is_available(self) -> bool:
        """Check if this runner's CLI is available."""
        pass

    def _execute_subprocess(
        self,
        cmd: list[str],
        timeout: Optional[int] = None
    ) -> tuple[str, str, int]:
        """Execute a subprocess and return stdout, stderr, returncode."""
        start_time = time.time()

        if self.debug:
            print(f"[{self.name}] Executing: {' '.join(cmd)}")

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout or self.timeout,
                cwd=self.working_dir
            )

            execution_time = time.time() - start_time

            if self.debug:
                print(f"[{self.name}] Completed in {execution_time:.2f}s")

            return result.stdout, result.stderr, result.returncode

        except subprocess.TimeoutExpired:
            return "", f"Timeout after {timeout or self.timeout}s", -1
        except FileNotFoundError:
            return "", f"Executable not found: {self.executable}", -1
        except Exception as e:
            return "", str(e), -1

    @property
    def session_id(self) -> Optional[str]:
        """Current session ID for conversation continuity."""
        return self._session_id

    def reset_session(self) -> None:
        """Reset the session to start fresh."""
        self._session_id = None
