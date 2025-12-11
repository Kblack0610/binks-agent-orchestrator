"""
OpenRouter Runner - Access 20+ free models via single API

OpenRouter provides a unified API for multiple LLM providers,
including many free models.

Usage:
    runner = OpenRouterRunner()  # Uses OPENROUTER_API_KEY env var
    result = runner.run("Hello!")

    # With specific model
    runner = OpenRouterRunner(model="meta-llama/llama-3.1-8b-instruct:free")

Free models (as of Dec 2024):
    - meta-llama/llama-3.1-8b-instruct:free
    - mistralai/mistral-7b-instruct:free
    - google/gemma-2-9b-it:free
    - nousresearch/hermes-3-llama-3.1-405b:free (limited)
    - And many more...
"""
import os
import time
from pathlib import Path
from typing import Optional, List

from .base import CLIRunner, RunnerResult


# OpenRouter API configuration
OPENROUTER_API_URL = "https://openrouter.ai/api/v1/chat/completions"

# Popular free models on OpenRouter (updated Dec 2024)
# Note: Free models change frequently - use list_free_models() for current list
FREE_MODELS = {
    "qwen/qwen3-coder:free": "Qwen3 Coder 480B - coding focused",
    "nvidia/nemotron-nano-9b-v2:free": "NVIDIA Nemotron Nano 9B - fast, capable",
    "amazon/nova-2-lite-v1:free": "Amazon Nova 2 Lite - general purpose",
    "mistralai/devstral-2512:free": "Mistral Devstral - development tasks",
    "openai/gpt-oss-20b:free": "OpenAI GPT-OSS 20B - open source",
    "z-ai/glm-4.5-air:free": "GLM 4.5 Air - multilingual",
}

# Paid models (for reference, require credits)
PAID_MODELS = {
    "anthropic/claude-3.5-sonnet": "Claude 3.5 Sonnet",
    "openai/gpt-4o": "GPT-4o",
    "google/gemini-pro-1.5": "Gemini Pro 1.5",
}

DEFAULT_MODEL = "nvidia/nemotron-nano-9b-v2:free"


class OpenRouterRunner(CLIRunner):
    """
    Runner for OpenRouter API - unified access to many LLMs.

    OpenRouter aggregates multiple LLM providers behind a single API.
    Many free models available with generous rate limits.
    """

    def __init__(
        self,
        model: str = DEFAULT_MODEL,
        api_key: Optional[str] = None,
        temperature: float = 0.7,
        max_tokens: int = 4096,
        site_url: Optional[str] = None,
        site_name: Optional[str] = None,
        timeout: int = 120,
        debug: bool = False
    ):
        """
        Initialize OpenRouter runner.

        Args:
            model: Model to use (see FREE_MODELS for free options)
            api_key: OpenRouter API key (or set OPENROUTER_API_KEY env var)
            temperature: Sampling temperature (0.0-2.0)
            max_tokens: Maximum tokens in response
            site_url: Your site URL (for OpenRouter leaderboards)
            site_name: Your site name (for OpenRouter leaderboards)
            timeout: Request timeout in seconds
            debug: Enable debug output
        """
        super().__init__(
            name="openrouter",
            executable="openrouter",  # Not used, but required by base
            timeout=timeout,
            debug=debug
        )

        self.model = model
        self.api_key = api_key if api_key is not None else os.getenv("OPENROUTER_API_KEY")
        self.temperature = temperature
        self.max_tokens = max_tokens
        self.site_url = site_url or "https://github.com/binks-agent-orchestrator"
        self.site_name = site_name or "Binks Agent Orchestrator"

    def is_available(self) -> bool:
        """Check if OpenRouter API is available (has API key)."""
        if not self.api_key:
            return False

        try:
            import requests
            response = requests.get(
                "https://openrouter.ai/api/v1/models",
                headers={"Authorization": f"Bearer {self.api_key}"},
                timeout=5
            )
            return response.status_code == 200
        except:
            # If we have a key, assume it's valid
            return True

    def run(self, prompt: str, **kwargs) -> RunnerResult:
        """
        Run a prompt through OpenRouter API.

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
                error="OPENROUTER_API_KEY not set",
                backend="openrouter"
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
                OPENROUTER_API_URL,
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                    "Content-Type": "application/json",
                    "HTTP-Referer": self.site_url,
                    "X-Title": self.site_name,
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
                error_data = response.json()
                error_msg = error_data.get("error", {}).get("message", response.text)
                return RunnerResult(
                    content="",
                    success=False,
                    error=f"OpenRouter API error: {error_msg}",
                    backend="openrouter",
                    model=model,
                    execution_time=execution_time
                )

            data = response.json()
            content = data["choices"][0]["message"]["content"]
            usage = data.get("usage", {})

            if self.debug:
                print(f"[OpenRouter] Model: {model}")
                print(f"[OpenRouter] Tokens: {usage.get('total_tokens', 'N/A')}")
                print(f"[OpenRouter] Time: {execution_time:.2f}s")

            return RunnerResult(
                content=content,
                success=True,
                backend="openrouter",
                model=model,
                execution_time=execution_time,
                tokens_used=usage.get("total_tokens"),
                metadata={
                    "prompt_tokens": usage.get("prompt_tokens"),
                    "completion_tokens": usage.get("completion_tokens"),
                    "openrouter_id": data.get("id"),
                }
            )

        except requests.exceptions.Timeout:
            return RunnerResult(
                content="",
                success=False,
                error=f"OpenRouter API timeout after {self.timeout}s",
                backend="openrouter",
                model=model,
                execution_time=time.time() - start_time
            )
        except Exception as e:
            return RunnerResult(
                content="",
                success=False,
                error=f"OpenRouter API error: {str(e)}",
                backend="openrouter",
                model=model,
                execution_time=time.time() - start_time
            )

    @classmethod
    def list_free_models(cls) -> dict:
        """List available free models."""
        return FREE_MODELS.copy()

    @classmethod
    def list_all_models(cls) -> dict:
        """List all known models (free + paid)."""
        return {**FREE_MODELS, **PAID_MODELS}

    def fetch_models(self) -> List[dict]:
        """Fetch current model list from OpenRouter API."""
        if not self.api_key:
            return []

        try:
            import requests
            response = requests.get(
                "https://openrouter.ai/api/v1/models",
                headers={"Authorization": f"Bearer {self.api_key}"},
                timeout=10
            )
            if response.status_code == 200:
                return response.json().get("data", [])
        except:
            pass

        return []
