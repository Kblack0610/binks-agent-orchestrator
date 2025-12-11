"""
JobScan AI API Client

Uses the JobScan AI actor on Apify to score job descriptions.
Also supports local LLM-based scoring as fallback.
"""
import os
import json
import time
import hashlib
from pathlib import Path
from typing import Optional, List, Dict, Any
from datetime import datetime
import sys

# Add orchestrator to path for LLM fallback
ORCHESTRATOR_PATH = Path(__file__).parent.parent.parent / "orchestrator" / "cli_orchestrator"
sys.path.insert(0, str(ORCHESTRATOR_PATH))

from .models import (
    JobScore, JobAnalysis, ResumeProfile,
    KeywordMatch, MatchLevel
)


# Apify configuration
APIFY_API_URL = "https://api.apify.com/v2"
JOBSCAN_ACTOR_ID = "bluelightco/jobscan-ai"


class JobScanClient:
    """
    Client for JobScan AI scoring.

    Supports two modes:
    1. Apify API mode (requires APIFY_TOKEN env var)
    2. Local LLM mode (uses CLI orchestrator backends)

    Usage:
        # API mode (if APIFY_TOKEN set)
        client = JobScanClient()

        # Force LLM mode
        client = JobScanClient(mode="llm")

        # Score a job
        result = client.analyze_job(
            job_description="...",
            job_title="Software Engineer",
            company="TechCorp",
            resume=profile
        )
    """

    def __init__(
        self,
        mode: str = "auto",
        apify_token: Optional[str] = None,
        cache_dir: Optional[Path] = None,
        debug: bool = False
    ):
        """
        Initialize client.

        Args:
            mode: "auto" (detect), "api" (Apify), or "llm" (local)
            apify_token: Apify API token (or set APIFY_TOKEN env var)
            cache_dir: Directory for caching results
            debug: Enable debug output
        """
        self.debug = debug
        self.apify_token = apify_token or os.getenv("APIFY_TOKEN")

        # Determine mode
        if mode == "auto":
            self.mode = "api" if self.apify_token else "llm"
        else:
            self.mode = mode

        # Validate API mode
        if self.mode == "api" and not self.apify_token:
            raise ValueError("APIFY_TOKEN required for API mode")

        # Set up caching
        if cache_dir:
            self.cache_dir = Path(cache_dir)
        else:
            self.cache_dir = Path.home() / ".jobscan_cache"
        self.cache_dir.mkdir(parents=True, exist_ok=True)

        # LLM runner (lazy init)
        self._runner = None

        if self.debug:
            print(f"JobScanClient initialized in {self.mode} mode")

    def _get_runner(self):
        """Get LLM runner for local scoring."""
        if self._runner is None:
            from runners import ClaudeRunner, GeminiRunner

            claude = ClaudeRunner()
            if claude.is_available():
                self._runner = claude
            else:
                gemini = GeminiRunner(backend="gemini")
                if gemini.is_available():
                    self._runner = gemini
                else:
                    gemini_api = GeminiRunner(backend="api")
                    if gemini_api.is_available():
                        self._runner = gemini_api
                    else:
                        raise RuntimeError("No LLM backend available for local scoring")

        return self._runner

    def _cache_key(self, job_desc: str, resume_text: str) -> str:
        """Generate cache key from job + resume."""
        content = f"{job_desc}|||{resume_text}"
        return hashlib.sha256(content.encode()).hexdigest()[:16]

    def _get_cached(self, cache_key: str) -> Optional[Dict]:
        """Get cached result if exists."""
        cache_file = self.cache_dir / f"{cache_key}.json"
        if cache_file.exists():
            with open(cache_file) as f:
                data = json.load(f)
                # Check if cache is less than 7 days old
                cached_at = datetime.fromisoformat(data.get("cached_at", "2000-01-01"))
                if (datetime.now() - cached_at).days < 7:
                    return data
        return None

    def _save_cache(self, cache_key: str, data: Dict):
        """Save result to cache."""
        data["cached_at"] = datetime.now().isoformat()
        cache_file = self.cache_dir / f"{cache_key}.json"
        with open(cache_file, "w") as f:
            json.dump(data, f, indent=2)

    def analyze_job(
        self,
        job_description: str,
        job_title: str,
        company: str,
        resume: ResumeProfile,
        job_url: Optional[str] = None,
        use_cache: bool = True
    ) -> JobAnalysis:
        """
        Analyze a job description against a resume.

        Args:
            job_description: Full job description text
            job_title: Job title
            company: Company name
            resume: ResumeProfile with resume text
            job_url: Optional URL to job posting
            use_cache: Whether to use cached results

        Returns:
            JobAnalysis with scoring results
        """
        start_time = time.time()

        # Check cache
        cache_key = self._cache_key(job_description, resume.resume_text)
        if use_cache:
            cached = self._get_cached(cache_key)
            if cached:
                if self.debug:
                    print(f"Using cached result for {cache_key}")
                return JobAnalysis.from_dict(cached)

        # Create base analysis
        analysis = JobAnalysis(
            job_title=job_title,
            company=company,
            job_url=job_url,
            job_description=job_description,
            analysis_id=cache_key
        )

        try:
            if self.mode == "api":
                score = self._score_via_api(job_description, resume.resume_text)
            else:
                score = self._score_via_llm(job_description, resume.resume_text)

            analysis.score = score

            # Cache successful result
            if use_cache:
                self._save_cache(cache_key, analysis.to_dict())

        except Exception as e:
            analysis.error = str(e)
            if self.debug:
                import traceback
                traceback.print_exc()

        return analysis

    def _score_via_api(self, job_description: str, resume_text: str) -> JobScore:
        """Score using Apify JobScan AI actor."""
        import requests

        # Start actor run
        run_url = f"{APIFY_API_URL}/acts/{JOBSCAN_ACTOR_ID}/runs"
        headers = {"Authorization": f"Bearer {self.apify_token}"}

        response = requests.post(
            run_url,
            headers=headers,
            json={
                "jobDescription": job_description,
                "resume": resume_text,
            },
            timeout=60
        )
        response.raise_for_status()
        run_data = response.json()["data"]
        run_id = run_data["id"]

        # Wait for completion
        status_url = f"{APIFY_API_URL}/actor-runs/{run_id}"
        for _ in range(60):  # Max 60 attempts (5 min)
            time.sleep(5)
            status_response = requests.get(status_url, headers=headers)
            status = status_response.json()["data"]["status"]

            if status == "SUCCEEDED":
                break
            elif status in ["FAILED", "ABORTED", "TIMED-OUT"]:
                raise RuntimeError(f"Actor run {status}")

        # Get results
        results_url = f"{APIFY_API_URL}/actor-runs/{run_id}/dataset/items"
        results_response = requests.get(results_url, headers=headers)
        results = results_response.json()

        if not results:
            raise RuntimeError("No results from JobScan actor")

        return self._parse_api_response(results[0])

    def _parse_api_response(self, data: Dict) -> JobScore:
        """Parse JobScan API response into JobScore."""
        # Extract scores (adjust based on actual API response)
        overall = data.get("overallScore", data.get("score", 50))
        keywords = data.get("keywordScore", overall)
        skills = data.get("skillsScore", overall)
        experience = data.get("experienceScore", overall)
        education = data.get("educationScore", overall)

        # Parse keywords
        matched = []
        for kw in data.get("matchedKeywords", []):
            matched.append(KeywordMatch(
                keyword=kw.get("keyword", kw) if isinstance(kw, dict) else kw,
                level=MatchLevel.STRONG,
                category=kw.get("category", "") if isinstance(kw, dict) else ""
            ))

        missing = []
        for kw in data.get("missingKeywords", []):
            missing.append(KeywordMatch(
                keyword=kw.get("keyword", kw) if isinstance(kw, dict) else kw,
                level=MatchLevel.MISSING,
                category=kw.get("category", "") if isinstance(kw, dict) else "",
                suggestions=kw.get("suggestions", []) if isinstance(kw, dict) else []
            ))

        return JobScore(
            overall_score=int(overall),
            keyword_score=int(keywords),
            skills_score=int(skills),
            experience_score=int(experience),
            education_score=int(education),
            matched_keywords=matched,
            missing_keywords=missing,
            recommendations=data.get("recommendations", []),
            formatting_issues=data.get("formattingIssues", [])
        )

    def _score_via_llm(self, job_description: str, resume_text: str) -> JobScore:
        """Score using local LLM."""
        runner = self._get_runner()

        prompt = f'''You are an ATS (Applicant Tracking System) analyzer. Score how well this resume matches the job description.

JOB DESCRIPTION:
---
{job_description[:4000]}
---

RESUME:
---
{resume_text[:4000]}
---

Analyze the match and respond with ONLY valid JSON (no markdown, no explanation outside JSON):
{{
    "overall_score": <0-100>,
    "keyword_score": <0-100>,
    "skills_score": <0-100>,
    "experience_score": <0-100>,
    "education_score": <0-100>,
    "matched_keywords": [
        {{"keyword": "Python", "category": "skill", "importance": "required"}},
        ...
    ],
    "missing_keywords": [
        {{"keyword": "Kubernetes", "category": "tool", "importance": "preferred", "suggestions": ["mention any container orchestration experience"]}},
        ...
    ],
    "recommendations": [
        "Add more quantifiable achievements",
        ...
    ],
    "formatting_issues": []
}}

Score guidelines:
- 80-100: Excellent match, strong candidate
- 60-79: Good match, worth applying
- 40-59: Fair match, consider tailoring resume
- 0-39: Poor match, significant gaps'''

        result = runner.run(prompt)
        if not result.success:
            raise RuntimeError(f"LLM error: {result.error}")

        # Parse JSON from response
        import re
        json_match = re.search(r'\{[\s\S]*\}', result.content)
        if not json_match:
            raise ValueError("No JSON found in LLM response")

        data = json.loads(json_match.group())
        return self._parse_api_response(data)

    def analyze_jobs(
        self,
        jobs: List[Dict[str, str]],
        resume: ResumeProfile
    ) -> List[JobAnalysis]:
        """
        Analyze multiple jobs.

        Args:
            jobs: List of dicts with job_description, job_title, company, job_url
            resume: ResumeProfile to match against

        Returns:
            List of JobAnalysis results
        """
        results = []
        for job in jobs:
            analysis = self.analyze_job(
                job_description=job.get("job_description", ""),
                job_title=job.get("job_title", "Unknown"),
                company=job.get("company", "Unknown"),
                job_url=job.get("job_url"),
                resume=resume
            )
            results.append(analysis)
        return results

    def get_recommendations(self, analysis: JobAnalysis) -> List[str]:
        """Get actionable recommendations from analysis."""
        if not analysis.score:
            return ["Unable to analyze - please retry"]

        recommendations = list(analysis.score.recommendations)

        # Add keyword-based recommendations
        missing_required = [
            k for k in analysis.score.missing_keywords
            if k.importance == "required"
        ]
        if missing_required:
            keywords = ", ".join(k.keyword for k in missing_required[:5])
            recommendations.insert(0, f"Add required keywords: {keywords}")

        # Add score-based recommendations
        if analysis.score.overall_score < 50:
            recommendations.append(
                "Consider tailoring your resume specifically for this role"
            )
        elif analysis.score.overall_score >= 80:
            recommendations.append(
                "Strong match! Apply with confidence"
            )

        return recommendations
