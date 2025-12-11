# Email Scoring LLM

## Overview
Use an LLM to automatically score and categorize job-related emails, predicting the likelihood of progressing to next steps in the application process.

## Use Case
Process incoming emails related to job applications:
1. Categorize email type (rejection, interview request, follow-up, etc.)
2. Score likelihood of moving forward (0-100)
3. Extract key information (dates, contacts, next steps)
4. Prioritize response queue

## Categories

| Category | Description | Score Range |
|----------|-------------|-------------|
| Interview Request | Explicit interview invitation | 90-100 |
| Positive Response | Interest expressed, next steps unclear | 70-89 |
| Information Request | Asking for more details/samples | 50-69 |
| Auto-Reply | Application received acknowledgment | 20-49 |
| Rejection | Clear rejection, role filled | 0-19 |
| Spam/Unrelated | Not job-related | N/A |

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Email Inbox    │ ──▶ │  Email Fetcher   │ ──▶ │  Scoring LLM    │
│  (Gmail/IMAP)   │     │  (IMAP/API)      │     │  (Claude/GPT)   │
└─────────────────┘     └──────────────────┘     └────────┬────────┘
                                                          │
                                                          ▼
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Dashboard      │ ◀── │  Results DB      │ ◀── │  Parsed Result  │
│  + Alerts       │     │  (SQLite/JSON)   │     │  (Score, Meta)  │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

## Implementation Tasks

### Phase 1: Email Integration
- [ ] Set up Gmail API or IMAP connection
- [ ] Filter for job-related emails (sender patterns, subjects)
- [ ] Extract email body (handle HTML vs plain text)

### Phase 2: LLM Scoring Pipeline
- [ ] Design scoring prompt with examples
- [ ] Create `EmailScorerRunner` using orchestrator
- [ ] Parse structured output (category, score, next_steps)

### Phase 3: Data Storage
- [ ] Store results with email metadata
- [ ] Track response rates by company/role
- [ ] Identify patterns in successful applications

### Phase 4: Notifications
- [ ] Alert on high-score emails (interview requests)
- [ ] Daily summary of new scored emails
- [ ] Integration with task manager (optional)

## Prompt Design

```
You are an email classifier for job applications.

Analyze this email and provide:
1. CATEGORY: [Interview Request | Positive Response | Information Request | Auto-Reply | Rejection | Spam]
2. SCORE: 0-100 likelihood of progressing to next interview/step
3. NEXT_STEPS: What action should be taken (if any)
4. KEY_INFO: Extract dates, contact names, deadlines

Email:
---
{email_content}
---

Respond in JSON format:
{
  "category": "...",
  "score": N,
  "next_steps": "...",
  "key_info": {...},
  "reasoning": "..."
}
```

## Example Output

```json
{
  "category": "Interview Request",
  "score": 95,
  "next_steps": "Reply to schedule interview for next week",
  "key_info": {
    "contact": "Sarah Miller",
    "deadline": "2024-01-15",
    "interview_type": "Technical phone screen",
    "duration": "45 minutes"
  },
  "reasoning": "Direct interview invitation with specific scheduling request"
}
```

## Integration with Orchestrator

```python
from orchestrator import Orchestrator
from agent import Agent, AgentRole, create_agent
from runners import ClaudeRunner

def score_email(email_content: str) -> dict:
    runner = ClaudeRunner()
    scorer = create_agent("email-scorer", runner, AgentRole.EXECUTOR)

    prompt = f"""[Email Scoring Prompt]

    Email:
    {email_content}
    """

    response = scorer.invoke(prompt)
    return parse_json_response(response.content)
```

## Privacy Considerations
- Process emails locally (don't store content in cloud)
- Only store scores and metadata, not full email content
- Option to exclude certain senders/domains

## Priority
**High** - Directly impacts job search efficiency and response times.

## Related Projects
- JobScan AI Integration (for application tracking)
- Calendar integration for interview scheduling
