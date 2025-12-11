#!/usr/bin/env python3
"""
CLI interface for Email Scorer service.

Usage:
    python -m services.email_scorer.cli score "Email body here"
    python -m services.email_scorer.cli score --file email.txt
    python -m services.email_scorer.cli score --subject "Interview" --sender "hr@company.com" "Body..."
    python -m services.email_scorer.cli batch emails.json
"""
import argparse
import json
import sys
from pathlib import Path

from .scorer import EmailScorer
from .models import EmailScore


def score_single(args):
    """Score a single email."""
    scorer = EmailScorer(backend=args.backend, debug=args.debug)

    # Get body from file or argument
    if args.file:
        body = Path(args.file).read_text()
    elif args.body:
        body = args.body
    else:
        # Read from stdin
        print("Enter email body (Ctrl+D to finish):", file=sys.stderr)
        body = sys.stdin.read()

    result = scorer.score_email(
        body=body,
        subject=args.subject or "",
        sender=args.sender or "",
        date=args.date or ""
    )

    if args.json:
        print(json.dumps(result.to_dict(), indent=2))
    else:
        print_result(result)


def score_batch(args):
    """Score multiple emails from a JSON file."""
    scorer = EmailScorer(backend=args.backend, debug=args.debug)

    with open(args.file) as f:
        emails = json.load(f)

    results = scorer.score_emails(emails)

    if args.json:
        output = [r.to_dict() for r in results]
        print(json.dumps(output, indent=2))
    else:
        for i, result in enumerate(results, 1):
            print(f"\n{'='*60}")
            print(f"Email {i}: {result.subject or 'No Subject'}")
            print('='*60)
            print_result(result)


def print_result(result: EmailScore):
    """Pretty print a scoring result."""
    # Category with color coding
    category_colors = {
        "interview_request": "\033[92m",  # Green
        "positive_response": "\033[92m",
        "scheduling": "\033[92m",
        "info_request": "\033[93m",        # Yellow
        "follow_up": "\033[93m",
        "auto_reply": "\033[90m",          # Gray
        "rejection": "\033[91m",           # Red
        "spam": "\033[91m",
        "unknown": "\033[90m",
    }
    reset = "\033[0m"
    color = category_colors.get(result.category.value, "")

    print(f"\n{color}Category: {result.category.value.upper()}{reset}")
    print(f"Score: {result.score}/100 ({result.priority})")
    print(f"Actionable: {'Yes' if result.is_actionable else 'No'}")
    print(f"\nNext Steps: {result.next_steps}")

    if result.key_info:
        print("\nKey Information:")
        for key, value in result.key_info.items():
            print(f"  - {key}: {value}")

    print(f"\nReasoning: {result.reasoning}")

    if result.backend_used:
        print(f"\n[Backend: {result.backend_used}, Time: {result.execution_time:.2f}s]")


def main():
    parser = argparse.ArgumentParser(
        description="Score job application emails using LLM"
    )
    parser.add_argument(
        "--backend", "-b",
        choices=["auto", "claude", "gemini"],
        default="auto",
        help="LLM backend to use (default: auto)"
    )
    parser.add_argument(
        "--debug", "-d",
        action="store_true",
        help="Enable debug output"
    )
    parser.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output as JSON"
    )

    subparsers = parser.add_subparsers(dest="command", help="Commands")

    # Score single email
    score_parser = subparsers.add_parser("score", help="Score a single email")
    score_parser.add_argument(
        "body",
        nargs="?",
        help="Email body text (or use --file)"
    )
    score_parser.add_argument(
        "--file", "-f",
        help="Read email body from file"
    )
    score_parser.add_argument(
        "--subject", "-s",
        help="Email subject"
    )
    score_parser.add_argument(
        "--sender",
        help="Email sender"
    )
    score_parser.add_argument(
        "--date",
        help="Email date"
    )
    score_parser.set_defaults(func=score_single)

    # Batch scoring
    batch_parser = subparsers.add_parser("batch", help="Score multiple emails")
    batch_parser.add_argument(
        "file",
        help="JSON file with array of email objects"
    )
    batch_parser.set_defaults(func=score_batch)

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    try:
        args.func(args)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        if args.debug:
            import traceback
            traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
