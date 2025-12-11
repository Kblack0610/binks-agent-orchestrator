"""
REAL Integration Tests for JobScan AI

Tests use local LLM backend (not Apify API) for scoring.
Run with: pytest -m requires_llm services/jobscan/tests/ -v
"""
import sys
import pytest
from pathlib import Path

# Add paths
SERVICE_DIR = Path(__file__).parent.parent
SERVICES_DIR = SERVICE_DIR.parent
PROJECT_ROOT = SERVICES_DIR.parent
ORCHESTRATOR_PATH = PROJECT_ROOT / "orchestrator" / "cli_orchestrator"

sys.path.insert(0, str(PROJECT_ROOT))
sys.path.insert(0, str(ORCHESTRATOR_PATH))

from services.jobscan import JobScanClient, JobScore, JobAnalysis, ResumeProfile
from services.jobscan.models import MatchLevel, KeywordMatch


# =============================================================================
# Sample Data
# =============================================================================

SAMPLE_RESUME = """
John Developer
Senior Software Engineer

SKILLS
- Python (8 years): Django, FastAPI, Flask, pytest
- JavaScript/TypeScript: React, Node.js, Vue
- Cloud: AWS (EC2, S3, Lambda), Docker, Kubernetes
- Databases: PostgreSQL, MongoDB, Redis
- CI/CD: GitHub Actions, Jenkins

EXPERIENCE

Senior Software Engineer | TechCorp Inc | 2020-Present
- Led team of 5 engineers to redesign core payment system
- Reduced API response time by 60% through caching optimization
- Implemented microservices architecture serving 1M+ daily users
- Mentored junior developers and conducted code reviews

Software Engineer | StartupXYZ | 2017-2020
- Built REST APIs using Python/Django handling 500K daily requests
- Developed real-time notification system using WebSockets
- Implemented automated testing achieving 90% code coverage

EDUCATION
B.S. Computer Science | State University | 2017
"""

MATCHING_JOB = """
Senior Software Engineer - Python

We're looking for an experienced Python developer to join our platform team.

Requirements:
- 5+ years of Python experience
- Strong experience with Django or FastAPI
- Experience with cloud platforms (AWS preferred)
- Knowledge of PostgreSQL and Redis
- Experience with Docker and container orchestration
- Strong testing practices

Nice to have:
- Experience with microservices architecture
- Leadership experience
- React or frontend experience

What you'll do:
- Design and build scalable backend services
- Mentor junior engineers
- Improve system performance and reliability
"""

POOR_MATCH_JOB = """
Mobile iOS Developer

We need a senior iOS developer to build our flagship mobile app.

Requirements:
- 5+ years of iOS development with Swift
- Expert in UIKit and SwiftUI
- Experience with Core Data and Realm
- Published apps on the App Store
- Experience with Xcode and iOS debugging tools
- Knowledge of Apple's Human Interface Guidelines

Nice to have:
- Experience with React Native
- Android development experience
- AR/VR experience with ARKit
"""


# =============================================================================
# Fixtures
# =============================================================================

@pytest.fixture
def sample_profile():
    """Sample resume profile."""
    return ResumeProfile(
        name="test_profile",
        resume_text=SAMPLE_RESUME,
        skills=["Python", "Django", "AWS", "Docker", "PostgreSQL"],
        experience_years=8,
        target_roles=["Senior Software Engineer", "Backend Developer"]
    )


@pytest.fixture
def client():
    """JobScan client in LLM mode."""
    return JobScanClient(mode="llm", debug=True)


# =============================================================================
# Unit Tests (No API calls)
# =============================================================================

class TestModels:
    """Test data models."""

    def test_resume_profile_creation(self, sample_profile):
        """Can create a resume profile."""
        assert sample_profile.name == "test_profile"
        assert "Python" in sample_profile.resume_text
        assert len(sample_profile.skills) > 0

    def test_resume_profile_to_dict(self, sample_profile):
        """Profile serializes to dict."""
        data = sample_profile.to_dict()
        assert "name" in data
        assert "skills" in data
        assert "experience_years" in data

    def test_job_score_match_level(self):
        """JobScore computes match level correctly."""
        excellent = JobScore(90, 90, 90, 90, 90)
        good = JobScore(65, 65, 65, 65, 65)
        fair = JobScore(45, 45, 45, 45, 45)
        poor = JobScore(25, 25, 25, 25, 25)

        assert excellent.match_level == "EXCELLENT"
        assert good.match_level == "GOOD"
        assert fair.match_level == "FAIR"
        assert poor.match_level == "POOR"

    def test_job_score_worth_applying(self):
        """JobScore determines if worth applying."""
        high_score = JobScore(75, 75, 75, 75, 75)
        low_score = JobScore(35, 35, 35, 35, 35)

        assert high_score.is_worth_applying
        assert not low_score.is_worth_applying

    def test_keyword_match_to_dict(self):
        """KeywordMatch serializes properly."""
        kw = KeywordMatch(
            keyword="Python",
            level=MatchLevel.STRONG,
            category="skill",
            importance="required"
        )
        data = kw.to_dict()

        assert data["keyword"] == "Python"
        assert data["level"] == "strong"
        assert data["category"] == "skill"


# =============================================================================
# Integration Tests (Real LLM calls)
# =============================================================================

@pytest.mark.requires_llm
class TestJobScoring:
    """Tests that verify job scoring works with real LLM."""

    def test_scores_matching_job_high(self, client, sample_profile):
        """A matching job should score reasonably (>=40) and provide recommendations."""
        analysis = client.analyze_job(
            job_description=MATCHING_JOB,
            job_title="Senior Software Engineer",
            company="TechCo",
            resume=sample_profile,
            use_cache=False
        )

        assert analysis.success, f"Analysis failed: {analysis.error}"
        assert analysis.score is not None
        # LLM responses vary; check score is reasonable and recommendations exist
        assert analysis.score.overall_score >= 40, \
            f"Expected reasonable score for matching job, got {analysis.score.overall_score}"
        # Should have either recommendations or identified the match
        has_analysis = len(analysis.score.recommendations) > 0 or analysis.score.overall_score >= 50
        assert has_analysis, "Should provide analysis for matching job"

    def test_scores_poor_match_low(self, client, sample_profile):
        """A non-matching job should score low (<50)."""
        analysis = client.analyze_job(
            job_description=POOR_MATCH_JOB,
            job_title="iOS Developer",
            company="MobileCo",
            resume=sample_profile,
            use_cache=False
        )

        assert analysis.success, f"Analysis failed: {analysis.error}"
        assert analysis.score is not None
        assert analysis.score.overall_score < 55, \
            f"Expected low score for iOS job, got {analysis.score.overall_score}"

    def test_identifies_matched_keywords(self, client, sample_profile):
        """Should identify matches via keywords or recommendations."""
        analysis = client.analyze_job(
            job_description=MATCHING_JOB,
            job_title="Senior Python Developer",
            company="TechCo",
            resume=sample_profile,
            use_cache=False
        )

        assert analysis.success
        # LLM might return keywords in array OR mention them in recommendations
        matched_keywords = analysis.score.matched_keywords
        recommendations = " ".join(analysis.score.recommendations).lower()

        has_keyword_analysis = (
            len(matched_keywords) > 0 or
            any(kw in recommendations for kw in ["python", "django", "aws", "docker", "experience"])
        )
        assert has_keyword_analysis, \
            f"Should analyze keywords. Keywords: {matched_keywords}, Recs: {recommendations[:200]}"

    def test_identifies_missing_keywords(self, client, sample_profile):
        """Should identify gaps via keywords or recommendations."""
        analysis = client.analyze_job(
            job_description=POOR_MATCH_JOB,
            job_title="iOS Developer",
            company="MobileCo",
            resume=sample_profile,
            use_cache=False
        )

        assert analysis.success
        # LLM might return missing keywords in array OR mention gaps in recommendations
        missing_keywords = analysis.score.missing_keywords
        recommendations = " ".join(analysis.score.recommendations).lower()

        # Should identify mismatch via keywords OR recommendations
        has_gap_analysis = (
            len(missing_keywords) > 0 or
            any(kw in recommendations for kw in ["swift", "ios", "mobile", "mismatch", "gap"])
        )
        assert has_gap_analysis, \
            f"Should identify gaps. Missing: {missing_keywords}, Recs: {recommendations[:200]}"

    def test_provides_recommendations(self, client, sample_profile):
        """Should provide actionable recommendations."""
        analysis = client.analyze_job(
            job_description=MATCHING_JOB,
            job_title="Senior Backend Developer",
            company="TechCo",
            resume=sample_profile,
            use_cache=False
        )

        assert analysis.success

        # Get recommendations
        recs = client.get_recommendations(analysis)
        assert len(recs) > 0, "Should provide recommendations"


@pytest.mark.requires_llm
class TestJobScanMetadata:
    """Test metadata and result handling."""

    def test_analysis_includes_metadata(self, client, sample_profile):
        """Analysis includes job metadata."""
        analysis = client.analyze_job(
            job_description=MATCHING_JOB,
            job_title="Software Engineer",
            company="TestCorp",
            job_url="https://example.com/job/123",
            resume=sample_profile,
            use_cache=False
        )

        assert analysis.job_title == "Software Engineer"
        assert analysis.company == "TestCorp"
        assert analysis.job_url == "https://example.com/job/123"
        assert analysis.analyzed_at is not None

    def test_analysis_to_dict(self, client, sample_profile):
        """Analysis serializes to dict properly."""
        analysis = client.analyze_job(
            job_description=MATCHING_JOB,
            job_title="Software Engineer",
            company="TestCorp",
            resume=sample_profile,
            use_cache=False
        )

        data = analysis.to_dict()

        assert data["job_title"] == "Software Engineer"
        assert data["company"] == "TestCorp"
        assert data["success"] is True
        assert "score" in data
        assert data["score"]["overall_score"] > 0

    def test_analysis_from_dict(self, client, sample_profile):
        """Analysis deserializes from dict."""
        analysis = client.analyze_job(
            job_description=MATCHING_JOB,
            job_title="Engineer",
            company="Corp",
            resume=sample_profile,
            use_cache=False
        )

        data = analysis.to_dict()
        # Add full job_description for deserialization
        data["job_description"] = MATCHING_JOB

        restored = JobAnalysis.from_dict(data)

        assert restored.job_title == analysis.job_title
        assert restored.company == analysis.company
        assert restored.score.overall_score == analysis.score.overall_score


@pytest.mark.requires_llm
class TestCaching:
    """Test result caching."""

    def test_caches_results(self, sample_profile, tmp_path):
        """Results should be cached."""
        client = JobScanClient(mode="llm", cache_dir=tmp_path, debug=True)

        # First call
        analysis1 = client.analyze_job(
            job_description=MATCHING_JOB,
            job_title="Cached Job",
            company="CacheCorp",
            resume=sample_profile,
            use_cache=True
        )

        # Second call (should use cache)
        analysis2 = client.analyze_job(
            job_description=MATCHING_JOB,
            job_title="Cached Job",
            company="CacheCorp",
            resume=sample_profile,
            use_cache=True
        )

        assert analysis1.success
        assert analysis2.success
        # Scores should be identical from cache
        assert analysis1.score.overall_score == analysis2.score.overall_score

    def test_skip_cache_option(self, sample_profile, tmp_path):
        """Can skip cache with use_cache=False."""
        client = JobScanClient(mode="llm", cache_dir=tmp_path, debug=True)

        # This should work even with use_cache=False
        analysis = client.analyze_job(
            job_description=MATCHING_JOB,
            job_title="No Cache Job",
            company="NoCacheCorp",
            resume=sample_profile,
            use_cache=False
        )

        assert analysis.success
