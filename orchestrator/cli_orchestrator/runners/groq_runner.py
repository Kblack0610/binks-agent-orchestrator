"""
Groq Runner - Fast LLM inference via Groq API

Groq provides extremely fast inference for open-source models.
Free tier has generous rate limits.

Usage:
    runner = GroqRunner()  # Uses GROQ_API_KEY env var
    result = runner.run("Hello!")

    # With specific model
    runner = GroqRunner(model="llama-3.3-70b-versatile")

Available models (as of Dec 2024):
    - llama-3.3-70b-versatile (default, best quality)
    - llama-3.1-8b-instant (fastest)
    - llama-3.1-70b-versatile
    - mixtral-8x7b-32768
    - gemma2-9b-it
"""
import os
import time
from pathlib import Path
from typing import Optional

from .base import CLIRunner, RunnerResult


# Groq API configuration
GROQ_API_URL = "https://api.groq.com/openai/v1/chat/completions"

# Available Groq models
GROQ_MODELS = {
    "llama-3.3-70b-versatile": "Meta Llama 3.3 70B - best quality",
    "llama-3.1-8b-instant": "Meta Llama 3.1 8B - fastest",
    "llama-3.1-70b-versatile": "Meta Llama 3.1 70B",
    "mixtral-8x7b-32768": "Mistral Mixtral 8x7B - 32K context",
    "gemma2-9b-it": "Google Gemma 2 9B",
}

DEFAULT_MODEL = "llama-3.3-70b-versatile"


class GroqRunner(CLIRunner):
    """
    Runner for Groq API - ultra-fast LLM inference.

    Groq uses specialized hardware (LPUs) for extremely fast inference.
    Great for multi-step agents where latency matters.
    """

    def __init__(
        self,
        model: str = DEFAULT_MODEL,
        api_key: Optional[str] = None,
        temperature: float = 0.7,
        max_tokens: int = 4096,
        timeout: int = 60,
        debug: bool = False
    ):
        """
        Initialize Groq runner.

        Args:
            model: Groq model to use (see GROQ_MODELS)
            api_key: Groq API key (or set GROQ_API_KEY env var)
            temperature: Sampling temperature (0.0-2.0)
            max_tokens: Maximum tokens in response
            timeout: Request timeout in seconds
            debug: Enable debug output
        """
        super().__init__(
            name="groq",
            executable="groq",  # Not used, but required by base
            timeout=timeout,
            debug=debug
        )

        self.model = model
        self.api_key = api_key or os.getenv("GROQ_API_KEY")
        self.temperature = temperature
        self.max_tokens = max_tokens

    def is_available(self) -> bool:
        """Check if Groq API is available (has API key)."""
        if not self.api_key:
            return False

        # Optionally verify the key works
        try:
            import requests
            response = requests.get(
                "https://api.groq.com/openai/v1/models",
                headers={"Authorization": f"Bearer {self.api_key}"},
                timeout=5
            )
            return response.status_code == 200
        except:
            # If we have a key, assume it's valid
            return True

    def run(self, prompt: str, **kwargs) -> RunnerResult:
        """
        Run a prompt through Groq API.

        Args:
            prompt: The prompt to send
            **kwargs: Additional options
                - model: Override default model
                - temperature: Override temperature
                - max_tokens: Override max tokens
                - system: System message

        Returns:
            RunnerResult with response
        """
        if not self.api_key:
            return RunnerResult(
                content="",
                success=False,
                error="GROQ_API_KEY not set",
                backend="groq"
            )

        start_time = time.time()

        # Build request
        model = kwargs.get("model", self.model)
        temperature = kwargs.get("temperature", self.temperature)
        max_tokens = kwargs.get("max_tokens", self.max_tokens)
        system_msg = kwargs.get("system", "You are a helpful assistant.")

        messages = [
            {"role": "system", "content": system_msg},
            {"role": "user", "content": prompt}
        ]

        try:
            import requests

            response = requests.post(
                GROQ_API_URL,
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                    "Content-Type": "application/json"
                },
                json={
                    "model": model,
                    "messages": messages,
                    "temperature": temperature,
                    "max_tokens": max_tokens,
                },
                timeout=self.timeout
            )

            execution_time = time.time() - start_time

            if response.status_code != 200:
                error_msg = response.json().get("error", {}).get("message", response.text)
                return RunnerResult(
                    content="",
                    success=False,
                    error=f"Groq API error: {error_msg}",
                    backend="groq",
                    model=model,
                    execution_time=execution_time
                )

            data = response.json()
            content = data["choices"][0]["message"]["content"]
            usage = data.get("usage", {})

            if self.debug:
                print(f"[Groq] Model: {model}")
                print(f"[Groq] Tokens: {usage.get('total_tokens', 'N/A')}")
                print(f"[Groq] Time: {execution_time:.2f}s")

            return RunnerResult(
                content=content,
                success=True,
                backend="groq",
                model=model,
                execution_time=execution_time,
                tokens_used=usage.get("total_tokens"),
                metadata={
                    "prompt_tokens": usage.get("prompt_tokens"),
                    "completion_tokens": usage.get("completion_tokens"),
                    "groq_id": data.get("id"),
                }
            )

        except requests.exceptions.Timeout:
            return RunnerResult(
                content="",
                success=False,
                error=f"Groq API timeout after {self.timeout}s",
                backend="groq",
                model=model,
                execution_time=time.time() - start_time
            )
        except Exception as e:
            return RunnerResult(
                content="",
                success=False,
                error=f"Groq API error: {str(e)}",
                backend="groq",
                model=model,
                execution_time=time.time() - start_time
            )

    @classmethod
    def list_models(cls) -> dict:
        """List available Groq models."""
        return GROQ_MODELS.copy()
