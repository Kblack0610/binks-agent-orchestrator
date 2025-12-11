# JobScan AI Integration

## Overview
Integrate JobScan AI API to automatically score job descriptions from LinkedIn against your resume for ATS (Applicant Tracking System) compatibility.

## API Source
- **Provider**: Apify (bluelightco/jobscan-ai)
- **API Docs**: https://apify.com/bluelightco/jobscan-ai/api

## Use Case
When browsing LinkedIn jobs, automatically:
1. Extract job description text
2. Send to JobScan AI API with your resume
3. Receive ATS compatibility score
4. Store results for tracking/comparison

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  LinkedIn Job   │ ──▶ │  Extraction      │ ──▶ │  JobScan AI     │
│  Descriptions   │     │  Service         │     │  API (Apify)    │
└─────────────────┘     └──────────────────┘     └────────┬────────┘
                                                          │
                                                          ▼
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Dashboard/     │ ◀── │  Results         │ ◀── │  ATS Score      │
│  Reports        │     │  Storage         │     │  + Feedback     │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

## Implementation Tasks

### Phase 1: API Integration
- [ ] Set up Apify account and get API key
- [ ] Create JobScan API client wrapper
- [ ] Test with sample job descriptions
- [ ] Handle rate limits and errors

### Phase 2: Job Description Extraction
- [ ] LinkedIn job page scraper (or manual input)
- [ ] Parse and clean job description text
- [ ] Extract key requirements/keywords

### Phase 3: Results Management
- [ ] Store scores with job metadata (company, title, URL)
- [ ] Track score history over time
- [ ] Compare scores across similar roles

### Phase 4: Integration with Orchestrator
- [ ] Create `JobScanRunner` extending `CustomRunner`
- [ ] Use orchestrator to batch process jobs
- [ ] Generate summary reports

## API Request Example

```python
import requests

APIFY_TOKEN = "your_token_here"
ACTOR_ID = "bluelightco/jobscan-ai"

def score_job(job_description: str, resume_text: str) -> dict:
    """Score a job description against resume."""
    response = requests.post(
        f"https://api.apify.com/v2/acts/{ACTOR_ID}/runs",
        headers={"Authorization": f"Bearer {APIFY_TOKEN}"},
        json={
            "jobDescription": job_description,
            "resume": resume_text,
        }
    )
    return response.json()
```

## Expected Output
- Overall ATS score (0-100)
- Keyword match analysis
- Missing skills/keywords
- Formatting suggestions
- Actionable recommendations

## Cost Considerations
- Apify pricing model (per actor run)
- Estimate: $X per 100 job scans
- Consider caching similar job descriptions

## Priority
**Medium** - Useful for job search optimization but not blocking other work.

## Related Projects
- Email Scoring LLM (for tracking application responses)
- Job Application Tracker (potential future project)
