"""
JobScan AI Integration Service

Scores job descriptions against your resume for ATS compatibility
using the JobScan AI API via Apify.
"""

from .client import JobScanClient
from .models import JobScore, JobAnalysis, ResumeProfile

__all__ = ["JobScanClient", "JobScore", "JobAnalysis", "ResumeProfile"]
