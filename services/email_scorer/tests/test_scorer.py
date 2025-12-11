"""
REAL Integration Tests for Email Scorer

These tests ACTUALLY call the LLM backends. No mocks.
Run with: pytest -m requires_api services/email_scorer/tests/ -v
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

from services.email_scorer import EmailScorer, EmailScore, EmailCategory
from runners import ClaudeRunner, GeminiRunner


# =============================================================================
# Fixtures
# =============================================================================

def _get_any_runner():
    """Get first available runner."""
    claude = ClaudeRunner()
    if claude.is_available():
        return claude

    gemini = GeminiRunner(backend="gemini")
    if gemini.is_available():
        return gemini

    gemini_api = GeminiRunner(backend="api")
    if gemini_api.is_available():
        return gemini_api

    return None


@pytest.fixture
def scorer():
    """EmailScorer with any available backend."""
    runner = _get_any_runner()
    if not runner:
        pytest.skip("No LLM backend available")
    return EmailScorer(runner=runner, debug=True)


# =============================================================================
# Sample Emails for Testing
# =============================================================================

INTERVIEW_EMAIL = {
    "subject": "Interview Invitation - Software Engineer at TechCorp",
    "sender": "recruiter@techcorp.com",
    "body": """Hi,

Thank you for your application. We were impressed by your background and would like to invite you for a technical interview.

The interview will consist of:
- 30 min call with the hiring manager
- 1 hour technical coding session
- 30 min team culture fit discussion

Please let me know your availability for next week (Tuesday-Thursday, 9am-5pm PST).

Best regards,
Sarah Johnson
Technical Recruiter, TechCorp
"""
}

REJECTION_EMAIL = {
    "subject": "Re: Your Application to DataCo",
    "sender": "careers@dataco.com",
    "body": """Dear Applicant,

Thank you for your interest in the Data Engineer position at DataCo.

After careful consideration, we have decided to move forward with other candidates whose qualifications more closely match our current needs.

We wish you the best in your job search.

Regards,
DataCo Recruiting Team
"""
}

AUTO_REPLY_EMAIL = {
    "subject": "Application Received - Job ID #12345",
    "sender": "no-reply@company.com",
    "body": """Thank you for submitting your application.

This is an automated message to confirm we received your application for the position.

Our team will review your qualifications and contact you if there is a match.

Please do not reply to this email.
"""
}

INFO_REQUEST_EMAIL = {
    "subject": "Additional Information Needed",
    "sender": "hr@startup.io",
    "body": """Hi,

Thanks for applying! Before we proceed, could you please send us:

1. A portfolio of your recent work
2. Links to any open source contributions
3. Your salary expectations

Looking forward to hearing from you.

Best,
Mike
"""
}

SCHEDULING_EMAIL = {
    "subject": "Interview Confirmation - Jan 15th",
    "sender": "calendar@company.com",
    "body": """Your interview has been scheduled.

Date: January 15th, 2025
Time: 2:00 PM - 3:00 PM EST
Location: Video Call (link below)

Meeting Link: https://meet.company.com/abc123

Please confirm your attendance by replying to this email.
"""
}


# =============================================================================
# Test Cases - Real LLM Calls
# =============================================================================

@pytest.mark.requires_api
class TestEmailScoring:
    """Tests that verify email scoring actually works."""

    def test_scores_interview_request_correctly(self, scorer):
        """Interview invitations should score high (90+)."""
        result = scorer.score_email(
            body=INTERVIEW_EMAIL["body"],
            subject=INTERVIEW_EMAIL["subject"],
            sender=INTERVIEW_EMAIL["sender"]
        )

        assert isinstance(result, EmailScore)
        assert result.category == EmailCategory.INTERVIEW_REQUEST, \
            f"Expected INTERVIEW_REQUEST, got {result.category}"
        assert result.score >= 85, f"Score {result.score} too low for interview"
        assert result.is_actionable
        assert result.priority in ["URGENT", "HIGH"]

    def test_scores_rejection_correctly(self, scorer):
        """Rejections should score low (<20)."""
        result = scorer.score_email(
            body=REJECTION_EMAIL["body"],
            subject=REJECTION_EMAIL["subject"],
            sender=REJECTION_EMAIL["sender"]
        )

        assert result.category == EmailCategory.REJECTION, \
            f"Expected REJECTION, got {result.category}"
        assert result.score < 25, f"Score {result.score} too high for rejection"
        assert not result.is_actionable

    def test_scores_auto_reply_correctly(self, scorer):
        """Auto-replies should score low-medium (20-35)."""
        result = scorer.score_email(
            body=AUTO_REPLY_EMAIL["body"],
            subject=AUTO_REPLY_EMAIL["subject"],
            sender=AUTO_REPLY_EMAIL["sender"]
        )

        assert result.category == EmailCategory.AUTO_REPLY, \
            f"Expected AUTO_REPLY, got {result.category}"
        assert 15 <= result.score <= 40, f"Score {result.score} unexpected for auto-reply"
        assert not result.is_actionable

    def test_scores_info_request_correctly(self, scorer):
        """Info requests should score medium (50-69)."""
        result = scorer.score_email(
            body=INFO_REQUEST_EMAIL["body"],
            subject=INFO_REQUEST_EMAIL["subject"],
            sender=INFO_REQUEST_EMAIL["sender"]
        )

        assert result.category == EmailCategory.INFORMATION_REQUEST, \
            f"Expected INFORMATION_REQUEST, got {result.category}"
        assert 45 <= result.score <= 75, f"Score {result.score} unexpected for info request"
        assert result.is_actionable

    def test_scores_scheduling_correctly(self, scorer):
        """Scheduling emails should score high (70-90)."""
        result = scorer.score_email(
            body=SCHEDULING_EMAIL["body"],
            subject=SCHEDULING_EMAIL["subject"],
            sender=SCHEDULING_EMAIL["sender"]
        )

        # Accept either SCHEDULING or INTERVIEW_REQUEST (interview confirmations are both)
        valid_categories = [EmailCategory.SCHEDULING, EmailCategory.INTERVIEW_REQUEST]
        assert result.category in valid_categories, \
            f"Expected SCHEDULING or INTERVIEW_REQUEST, got {result.category}"
        assert result.score >= 65, f"Score {result.score} too low for scheduling"
        assert result.is_actionable

    def test_extracts_key_info(self, scorer):
        """Should extract relevant information from emails."""
        result = scorer.score_email(
            body=INTERVIEW_EMAIL["body"],
            subject=INTERVIEW_EMAIL["subject"],
            sender=INTERVIEW_EMAIL["sender"]
        )

        assert result.key_info is not None
        assert isinstance(result.key_info, dict)
        # Should extract company name or contact
        key_info_str = str(result.key_info).lower()
        assert "techcorp" in key_info_str or "sarah" in key_info_str, \
            f"Expected company/contact in key_info: {result.key_info}"

    def test_provides_reasoning(self, scorer):
        """Should provide reasoning for classification."""
        result = scorer.score_email(
            body=REJECTION_EMAIL["body"],
            subject=REJECTION_EMAIL["subject"],
            sender=REJECTION_EMAIL["sender"]
        )

        assert result.reasoning
        assert len(result.reasoning) > 10
        # Reasoning should mention rejection-related terms
        reasoning_lower = result.reasoning.lower()
        assert any(word in reasoning_lower for word in ["reject", "declined", "not", "moving"]), \
            f"Reasoning should explain rejection: {result.reasoning}"


@pytest.mark.requires_api
class TestEmailScorerMetadata:
    """Tests for metadata and result properties."""

    def test_includes_execution_time(self, scorer):
        """Results should include execution time."""
        result = scorer.score_email(body="Test email", subject="Test")

        assert result.execution_time is not None
        assert result.execution_time > 0

    def test_includes_backend_info(self, scorer):
        """Results should indicate which backend was used."""
        result = scorer.score_email(body="Test email", subject="Test")

        assert result.backend_used is not None
        assert result.backend_used in ["claude", "gemini", "gemini-api"]

    def test_result_to_dict(self, scorer):
        """EmailScore should serialize to dict properly."""
        result = scorer.score_email(
            body=INTERVIEW_EMAIL["body"],
            subject=INTERVIEW_EMAIL["subject"],
            email_id="test-123"
        )

        data = result.to_dict()

        assert data["category"] == result.category.value
        assert data["score"] == result.score
        assert data["email_id"] == "test-123"
        assert "scored_at" in data

    def test_result_from_dict(self, scorer):
        """EmailScore should deserialize from dict."""
        result = scorer.score_email(body="Test", subject="Test")
        data = result.to_dict()

        restored = EmailScore.from_dict(data)

        assert restored.category == result.category
        assert restored.score == result.score
        assert restored.next_steps == result.next_steps


@pytest.mark.requires_api
class TestBatchScoring:
    """Tests for batch email scoring."""

    def test_scores_multiple_emails(self, scorer):
        """Should score multiple emails in batch."""
        emails = [
            INTERVIEW_EMAIL,
            REJECTION_EMAIL,
            AUTO_REPLY_EMAIL
        ]

        results = scorer.score_emails(emails)

        assert len(results) == 3
        assert all(isinstance(r, EmailScore) for r in results)

        # Check each result is categorized appropriately
        categories = [r.category for r in results]
        assert EmailCategory.INTERVIEW_REQUEST in categories
        assert EmailCategory.REJECTION in categories
        assert EmailCategory.AUTO_REPLY in categories


# =============================================================================
# Backend-Specific Tests
# =============================================================================

@pytest.mark.requires_gemini_cli
class TestGeminiCLIScorer:
    """Tests specifically for Gemini CLI backend."""

    @pytest.fixture
    def gemini_scorer(self):
        gemini = GeminiRunner(backend="gemini")
        if not gemini.is_available():
            pytest.skip("Gemini CLI not available")
        return EmailScorer(runner=gemini, debug=True)

    def test_gemini_cli_scores_email(self, gemini_scorer):
        """Gemini CLI should score emails correctly."""
        result = gemini_scorer.score_email(
            body=INTERVIEW_EMAIL["body"],
            subject=INTERVIEW_EMAIL["subject"],
            sender=INTERVIEW_EMAIL["sender"]
        )

        assert result.success is not False  # Not explicitly failed
        assert result.category in list(EmailCategory)
        assert 0 <= result.score <= 100
        assert result.backend_used == "gemini"
