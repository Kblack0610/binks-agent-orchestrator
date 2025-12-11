"""
Data models for JobScan AI service.
"""
from dataclasses import dataclass, field
from datetime import datetime
from typing import Optional, Dict, List, Any
from enum import Enum
from pathlib import Path


class MatchLevel(Enum):
    """Match level for keywords/skills."""
    STRONG = "strong"       # Direct match
    PARTIAL = "partial"     # Related match
    MISSING = "missing"     # Not found


@dataclass
class KeywordMatch:
    """A single keyword match result."""
    keyword: str
    level: MatchLevel
    category: str = ""      # e.g., "skill", "tool", "qualification"
    importance: str = ""    # e.g., "required", "preferred"
    suggestions: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "keyword": self.keyword,
            "level": self.level.value,
            "category": self.category,
            "importance": self.importance,
            "suggestions": self.suggestions
        }


@dataclass
class JobScore:
    """Overall job-resume match score."""
    overall_score: int                          # 0-100 ATS compatibility
    keyword_score: int                          # Keyword match percentage
    skills_score: int                           # Skills match percentage
    experience_score: int                       # Experience relevance
    education_score: int                        # Education match

    # Match details
    matched_keywords: List[KeywordMatch] = field(default_factory=list)
    missing_keywords: List[KeywordMatch] = field(default_factory=list)

    # Recommendations
    recommendations: List[str] = field(default_factory=list)
    formatting_issues: List[str] = field(default_factory=list)

    @property
    def match_level(self) -> str:
        """Get human-readable match level."""
        if self.overall_score >= 80:
            return "EXCELLENT"
        elif self.overall_score >= 60:
            return "GOOD"
        elif self.overall_score >= 40:
            return "FAIR"
        else:
            return "POOR"

    @property
    def is_worth_applying(self) -> bool:
        """Is this job worth applying to?"""
        return self.overall_score >= 50

    def to_dict(self) -> Dict[str, Any]:
        return {
            "overall_score": self.overall_score,
            "keyword_score": self.keyword_score,
            "skills_score": self.skills_score,
            "experience_score": self.experience_score,
            "education_score": self.education_score,
            "match_level": self.match_level,
            "is_worth_applying": self.is_worth_applying,
            "matched_keywords": [k.to_dict() for k in self.matched_keywords],
            "missing_keywords": [k.to_dict() for k in self.missing_keywords],
            "recommendations": self.recommendations,
            "formatting_issues": self.formatting_issues
        }


@dataclass
class JobAnalysis:
    """Complete analysis of a job description."""
    job_title: str
    company: str
    job_url: Optional[str]
    job_description: str

    # Scoring result
    score: Optional[JobScore] = None

    # Metadata
    analyzed_at: datetime = field(default_factory=datetime.now)
    analysis_id: Optional[str] = None
    source: str = "manual"                      # "linkedin", "indeed", "manual"

    # Raw API response
    raw_response: Optional[Dict[str, Any]] = None
    error: Optional[str] = None

    @property
    def success(self) -> bool:
        return self.score is not None and self.error is None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "job_title": self.job_title,
            "company": self.company,
            "job_url": self.job_url,
            "job_description": self.job_description[:500] + "..." if len(self.job_description) > 500 else self.job_description,
            "score": self.score.to_dict() if self.score else None,
            "analyzed_at": self.analyzed_at.isoformat(),
            "analysis_id": self.analysis_id,
            "source": self.source,
            "success": self.success,
            "error": self.error
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "JobAnalysis":
        """Create from dictionary."""
        score_data = data.get("score")
        score = None
        if score_data:
            matched = [
                KeywordMatch(
                    keyword=k["keyword"],
                    level=MatchLevel(k["level"]),
                    category=k.get("category", ""),
                    importance=k.get("importance", ""),
                    suggestions=k.get("suggestions", [])
                )
                for k in score_data.get("matched_keywords", [])
            ]
            missing = [
                KeywordMatch(
                    keyword=k["keyword"],
                    level=MatchLevel(k["level"]),
                    category=k.get("category", ""),
                    importance=k.get("importance", ""),
                    suggestions=k.get("suggestions", [])
                )
                for k in score_data.get("missing_keywords", [])
            ]
            score = JobScore(
                overall_score=score_data["overall_score"],
                keyword_score=score_data["keyword_score"],
                skills_score=score_data["skills_score"],
                experience_score=score_data["experience_score"],
                education_score=score_data["education_score"],
                matched_keywords=matched,
                missing_keywords=missing,
                recommendations=score_data.get("recommendations", []),
                formatting_issues=score_data.get("formatting_issues", [])
            )

        return cls(
            job_title=data["job_title"],
            company=data["company"],
            job_url=data.get("job_url"),
            job_description=data["job_description"],
            score=score,
            analyzed_at=datetime.fromisoformat(data["analyzed_at"]) if data.get("analyzed_at") else datetime.now(),
            analysis_id=data.get("analysis_id"),
            source=data.get("source", "manual"),
            error=data.get("error")
        )


@dataclass
class ResumeProfile:
    """User's resume profile for matching."""
    name: str
    resume_text: str
    resume_file: Optional[Path] = None

    # Parsed components
    skills: List[str] = field(default_factory=list)
    experience_years: int = 0
    education: str = ""
    target_roles: List[str] = field(default_factory=list)

    created_at: datetime = field(default_factory=datetime.now)
    updated_at: datetime = field(default_factory=datetime.now)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "resume_text": self.resume_text[:200] + "...",
            "resume_file": str(self.resume_file) if self.resume_file else None,
            "skills": self.skills,
            "experience_years": self.experience_years,
            "education": self.education,
            "target_roles": self.target_roles,
            "created_at": self.created_at.isoformat(),
            "updated_at": self.updated_at.isoformat()
        }

    @classmethod
    def from_file(cls, name: str, resume_path: Path) -> "ResumeProfile":
        """Create profile from resume file."""
        resume_text = resume_path.read_text()
        return cls(
            name=name,
            resume_text=resume_text,
            resume_file=resume_path
        )
