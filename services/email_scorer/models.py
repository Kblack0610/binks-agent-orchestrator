"""
Data models for Email Scoring service.
"""
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Optional, Dict, Any


class EmailCategory(Enum):
    """Categories for job-related emails."""
    INTERVIEW_REQUEST = "interview_request"  # Explicit interview invitation
    POSITIVE_RESPONSE = "positive_response"  # Interest expressed, next steps unclear
    INFORMATION_REQUEST = "info_request"     # Asking for more details/samples
    AUTO_REPLY = "auto_reply"                # Application received acknowledgment
    REJECTION = "rejection"                  # Clear rejection, role filled
    FOLLOW_UP = "follow_up"                  # Follow-up from recruiter
    SCHEDULING = "scheduling"                # Calendar/scheduling related
    SPAM = "spam"                            # Not job-related
    UNKNOWN = "unknown"                      # Could not categorize


@dataclass
class EmailScore:
    """Result of scoring an email."""
    category: EmailCategory
    score: int  # 0-100 likelihood of progressing
    next_steps: str
    key_info: Dict[str, Any]
    reasoning: str

    # Metadata
    email_id: Optional[str] = None
    subject: Optional[str] = None
    sender: Optional[str] = None
    received_at: Optional[datetime] = None
    scored_at: datetime = field(default_factory=datetime.now)
    backend_used: Optional[str] = None
    execution_time: Optional[float] = None

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for storage."""
        return {
            "category": self.category.value,
            "score": self.score,
            "next_steps": self.next_steps,
            "key_info": self.key_info,
            "reasoning": self.reasoning,
            "email_id": self.email_id,
            "subject": self.subject,
            "sender": self.sender,
            "received_at": self.received_at.isoformat() if self.received_at else None,
            "scored_at": self.scored_at.isoformat(),
            "backend_used": self.backend_used,
            "execution_time": self.execution_time
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "EmailScore":
        """Create from dictionary."""
        return cls(
            category=EmailCategory(data["category"]),
            score=data["score"],
            next_steps=data["next_steps"],
            key_info=data.get("key_info", {}),
            reasoning=data["reasoning"],
            email_id=data.get("email_id"),
            subject=data.get("subject"),
            sender=data.get("sender"),
            received_at=datetime.fromisoformat(data["received_at"]) if data.get("received_at") else None,
            scored_at=datetime.fromisoformat(data["scored_at"]) if data.get("scored_at") else datetime.now(),
            backend_used=data.get("backend_used"),
            execution_time=data.get("execution_time")
        )

    @property
    def is_actionable(self) -> bool:
        """Does this email require action?"""
        return self.category in [
            EmailCategory.INTERVIEW_REQUEST,
            EmailCategory.POSITIVE_RESPONSE,
            EmailCategory.INFORMATION_REQUEST,
            EmailCategory.SCHEDULING
        ]

    @property
    def priority(self) -> str:
        """Get priority level based on score."""
        if self.score >= 90:
            return "URGENT"
        elif self.score >= 70:
            return "HIGH"
        elif self.score >= 50:
            return "MEDIUM"
        elif self.score >= 20:
            return "LOW"
        return "NONE"
