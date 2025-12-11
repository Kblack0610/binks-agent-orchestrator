"""
Email Scorer - Uses LLM to score and categorize job emails.
"""
import json
import re
import time
from typing import Optional, Union
from pathlib import Path
import sys

# Add orchestrator to path
ORCHESTRATOR_PATH = Path(__file__).parent.parent.parent / "orchestrator" / "cli_orchestrator"
sys.path.insert(0, str(ORCHESTRATOR_PATH))

from runners import ClaudeRunner, GeminiRunner
from runners.base import CLIRunner

from .models import EmailScore, EmailCategory


SCORING_PROMPT = '''You are an email classifier for job applications.

Analyze this email and provide a JSON response with:
1. category: One of [interview_request, positive_response, info_request, auto_reply, rejection, follow_up, scheduling, spam, unknown]
2. score: 0-100 likelihood of progressing to next interview/step
3. next_steps: What action should be taken (if any)
4. key_info: Extract relevant info (dates, contact names, deadlines, company, role)
5. reasoning: Brief explanation of your classification

Category definitions:
- interview_request: Explicit invitation to interview (score 90-100)
- positive_response: Interest shown but next steps unclear (score 70-89)
- info_request: Asking for more details, samples, portfolio (score 50-69)
- follow_up: Recruiter following up on application (score 40-60)
- scheduling: Calendar or meeting scheduling (score 70-90)
- auto_reply: Application received acknowledgment (score 20-35)
- rejection: Clear rejection, role filled (score 0-19)
- spam: Not job-related (score 0)
- unknown: Cannot determine (score 30)

Email:
---
From: {sender}
Subject: {subject}
Date: {date}

{body}
---

Respond ONLY with valid JSON (no markdown, no explanation outside JSON):
{{"category": "...", "score": N, "next_steps": "...", "key_info": {{}}, "reasoning": "..."}}'''


class EmailScorer:
    """
    Score job-related emails using LLM.

    Usage:
        scorer = EmailScorer()  # Auto-detects best available backend
        result = scorer.score_email(subject="Interview Request", body="...")
    """

    def __init__(
        self,
        runner: Optional[CLIRunner] = None,
        backend: str = "auto",
        debug: bool = False
    ):
        """
        Initialize scorer.

        Args:
            runner: Specific runner to use (optional)
            backend: Backend to use if no runner provided:
                     "auto" - auto-detect best available
                     "claude" - use Claude
                     "gemini" - use Gemini CLI
            debug: Enable debug output
        """
        self.debug = debug

        if runner:
            self.runner = runner
        else:
            self.runner = self._get_runner(backend)

        if self.debug:
            print(f"EmailScorer using backend: {self.runner.name}")

    def _get_runner(self, backend: str) -> CLIRunner:
        """Get the best available runner."""
        if backend == "auto":
            # Try Claude first, then Gemini CLI
            claude = ClaudeRunner()
            if claude.is_available():
                return claude

            gemini = GeminiRunner(backend="gemini")
            if gemini.is_available():
                return gemini

            # Fallback to Gemini API
            gemini_api = GeminiRunner(backend="api")
            if gemini_api.is_available():
                return gemini_api

            raise RuntimeError("No LLM backend available. Install Claude or Gemini CLI.")

        elif backend == "claude":
            runner = ClaudeRunner()
            if not runner.is_available():
                raise RuntimeError("Claude not available")
            return runner

        elif backend == "gemini":
            runner = GeminiRunner(backend="gemini")
            if not runner.is_available():
                raise RuntimeError("Gemini CLI not available")
            return runner

        else:
            raise ValueError(f"Unknown backend: {backend}")

    def score_email(
        self,
        body: str,
        subject: str = "",
        sender: str = "",
        date: str = "",
        email_id: Optional[str] = None
    ) -> EmailScore:
        """
        Score a single email.

        Args:
            body: Email body text
            subject: Email subject
            sender: Sender email/name
            date: Date received
            email_id: Optional unique identifier

        Returns:
            EmailScore with category, score, and metadata
        """
        # Build prompt
        prompt = SCORING_PROMPT.format(
            sender=sender or "Unknown",
            subject=subject or "No Subject",
            date=date or "Unknown",
            body=body[:4000]  # Limit body length
        )

        # Run LLM
        start_time = time.time()
        result = self.runner.run(prompt)
        execution_time = time.time() - start_time

        if not result.success:
            # Return unknown category on error
            return EmailScore(
                category=EmailCategory.UNKNOWN,
                score=0,
                next_steps="Error scoring email",
                key_info={"error": result.error},
                reasoning=f"LLM error: {result.error}",
                email_id=email_id,
                subject=subject,
                sender=sender,
                backend_used=self.runner.name,
                execution_time=execution_time
            )

        # Parse response
        return self._parse_response(
            result.content,
            email_id=email_id,
            subject=subject,
            sender=sender,
            backend=self.runner.name,
            execution_time=execution_time
        )

    def _parse_response(
        self,
        content: str,
        email_id: Optional[str] = None,
        subject: Optional[str] = None,
        sender: Optional[str] = None,
        backend: Optional[str] = None,
        execution_time: Optional[float] = None
    ) -> EmailScore:
        """Parse LLM response into EmailScore."""
        try:
            # Try to extract JSON from response
            # Handle case where LLM wraps in markdown code block
            json_match = re.search(r'\{[\s\S]*\}', content)
            if not json_match:
                raise ValueError("No JSON found in response")

            data = json.loads(json_match.group())

            # Map category string to enum
            category_str = data.get("category", "unknown").lower().replace("-", "_")
            try:
                category = EmailCategory(category_str)
            except ValueError:
                category = EmailCategory.UNKNOWN

            return EmailScore(
                category=category,
                score=int(data.get("score", 0)),
                next_steps=data.get("next_steps", ""),
                key_info=data.get("key_info", {}),
                reasoning=data.get("reasoning", ""),
                email_id=email_id,
                subject=subject,
                sender=sender,
                backend_used=backend,
                execution_time=execution_time
            )

        except (json.JSONDecodeError, ValueError) as e:
            if self.debug:
                print(f"Parse error: {e}")
                print(f"Content: {content[:500]}")

            return EmailScore(
                category=EmailCategory.UNKNOWN,
                score=30,
                next_steps="Manual review needed",
                key_info={"parse_error": str(e)},
                reasoning=f"Could not parse LLM response: {content[:200]}",
                email_id=email_id,
                subject=subject,
                sender=sender,
                backend_used=backend,
                execution_time=execution_time
            )

    def score_emails(self, emails: list) -> list[EmailScore]:
        """
        Score multiple emails.

        Args:
            emails: List of dicts with body, subject, sender, date, email_id

        Returns:
            List of EmailScore results
        """
        results = []
        for email in emails:
            score = self.score_email(
                body=email.get("body", ""),
                subject=email.get("subject", ""),
                sender=email.get("sender", ""),
                date=email.get("date", ""),
                email_id=email.get("email_id")
            )
            results.append(score)
        return results
