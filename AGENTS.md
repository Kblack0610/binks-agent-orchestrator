# Binks Agent Repo Instructions

## Frontend Worktree Policy

- This repository is the source of truth for the Binks backend, agent runtime, MCP integration, and web API.
- The Binks frontend (`binks-chat`) does not live in this repository.
- When a task requires frontend changes, make those changes only in the dedicated platform worktree for Binks.
- Do not edit the main/shared platform checkout for Binks frontend work.

## Canonical Frontend Location

- Expected Binks frontend worktree: `~/dev/bnb/platform-binks-agent`
- Expected app path inside that worktree: `~/dev/bnb/platform-binks-agent/apps/binks-chat`

If the dedicated worktree does not exist yet, stop and ask the user to create or confirm it before making frontend changes.

## Working Rules

- Backend-only tasks: work only in `~/dev/home/binks-agent-orchestrator`
- Frontend-only tasks: work only in `~/dev/bnb/platform-binks-agent/apps/binks-chat`
- Full-stack tasks: update backend here first, then update the frontend in the dedicated platform worktree
- Never use `~/dev/bnb/platform` for Binks frontend changes unless the user explicitly overrides this policy

## Verification

- For backend changes, verify from this repository
- For frontend changes, verify from the dedicated platform worktree
- Report which repository or worktree was changed and what was verified in each location
