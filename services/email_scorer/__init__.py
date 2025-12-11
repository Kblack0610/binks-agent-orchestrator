"""
Email Scoring LLM Service

Scores and categorizes job application emails using LLMs.
"""

from .models import EmailScore, EmailCategory
from .scorer import EmailScorer

__all__ = ["EmailScorer", "EmailScore", "EmailCategory"]
