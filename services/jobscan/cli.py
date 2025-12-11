#!/usr/bin/env python3
"""
CLI interface for JobScan AI service.

Usage:
    # Score a job (reads resume from file)
    python -m services.jobscan.cli score --resume resume.txt --job-file job.txt

    # Score with inline job description
    python -m services.jobscan.cli score --resume resume.txt "Job description here..."

    # Interactive mode
    python -m services.jobscan.cli interactive --resume resume.txt

    # Batch scoring from JSON
    python -m services.jobscan.cli batch --resume resume.txt jobs.json
"""
import argparse
import json
import sys
from pathlib import Path

from .client import JobScanClient
from .models import JobAnalysis, ResumeProfile


def print_score(analysis: JobAnalysis, verbose: bool = False):
    """Pretty print job analysis result."""
    reset = "\033[0m"

    if not analysis.success:
        print(f"\033[91mError: {analysis.error}{reset}")
        return

    score = analysis.score

    # Color based on score
    if score.overall_score >= 80:
        color = "\033[92m"  # Green
    elif score.overall_score >= 60:
        color = "\033[93m"  # Yellow
    elif score.overall_score >= 40:
        color = "\033[33m"  # Orange-ish
    else:
        color = "\033[91m"  # Red

    print(f"\n{'='*60}")
    print(f"{analysis.job_title} at {analysis.company}")
    print('='*60)

    print(f"\n{color}Overall Score: {score.overall_score}/100 ({score.match_level}){reset}")
    print(f"Worth Applying: {'Yes ✓' if score.is_worth_applying else 'No ✗'}")

    print(f"\nBreakdown:")
    print(f"  Keywords:   {score.keyword_score}/100")
    print(f"  Skills:     {score.skills_score}/100")
    print(f"  Experience: {score.experience_score}/100")
    print(f"  Education:  {score.education_score}/100")

    if score.matched_keywords:
        print(f"\n✓ Matched Keywords ({len(score.matched_keywords)}):")
        for kw in score.matched_keywords[:10]:
            category = f" [{kw.category}]" if kw.category else ""
            print(f"  - {kw.keyword}{category}")
        if len(score.matched_keywords) > 10:
            print(f"  ... and {len(score.matched_keywords) - 10} more")

    if score.missing_keywords:
        print(f"\n✗ Missing Keywords ({len(score.missing_keywords)}):")
        for kw in score.missing_keywords[:10]:
            category = f" [{kw.category}]" if kw.category else ""
            importance = f" ({kw.importance})" if kw.importance else ""
            print(f"  - {kw.keyword}{category}{importance}")
            if kw.suggestions and verbose:
                for s in kw.suggestions[:2]:
                    print(f"    → {s}")
        if len(score.missing_keywords) > 10:
            print(f"  ... and {len(score.missing_keywords) - 10} more")

    if score.recommendations:
        print(f"\nRecommendations:")
        for rec in score.recommendations[:5]:
            print(f"  • {rec}")

    if analysis.job_url:
        print(f"\nJob URL: {analysis.job_url}")


def cmd_score(args):
    """Score a single job."""
    # Load resume
    resume_text = Path(args.resume).read_text()
    profile = ResumeProfile(name="default", resume_text=resume_text)

    # Load job description
    if args.job_file:
        job_desc = Path(args.job_file).read_text()
    elif args.job_description:
        job_desc = args.job_description
    else:
        print("Enter job description (Ctrl+D to finish):", file=sys.stderr)
        job_desc = sys.stdin.read()

    # Create client
    client = JobScanClient(mode=args.mode, debug=args.debug)

    # Analyze
    analysis = client.analyze_job(
        job_description=job_desc,
        job_title=args.title or "Unknown Position",
        company=args.company or "Unknown Company",
        job_url=args.url,
        resume=profile,
        use_cache=not args.no_cache
    )

    if args.json:
        print(json.dumps(analysis.to_dict(), indent=2))
    else:
        print_score(analysis, verbose=args.verbose)


def cmd_batch(args):
    """Score multiple jobs from JSON file."""
    # Load resume
    resume_text = Path(args.resume).read_text()
    profile = ResumeProfile(name="default", resume_text=resume_text)

    # Load jobs
    with open(args.file) as f:
        jobs = json.load(f)

    client = JobScanClient(mode=args.mode, debug=args.debug)

    # Analyze all
    results = client.analyze_jobs(jobs, profile)

    if args.json:
        output = [r.to_dict() for r in results]
        print(json.dumps(output, indent=2))
    else:
        # Summary table
        print("\n" + "="*80)
        print(f"{'Job Title':<30} {'Company':<20} {'Score':>6} {'Match':>10}")
        print("="*80)

        for r in sorted(results, key=lambda x: x.score.overall_score if x.score else 0, reverse=True):
            if r.success:
                print(f"{r.job_title[:28]:<30} {r.company[:18]:<20} {r.score.overall_score:>5}% {r.score.match_level:>10}")
            else:
                print(f"{r.job_title[:28]:<30} {r.company[:18]:<20} {'ERROR':>6} {'-':>10}")

        print("="*80)

        # Show top recommendations
        top_jobs = [r for r in results if r.success and r.score.overall_score >= 60]
        if top_jobs:
            print(f"\nTop {min(3, len(top_jobs))} matches:")
            for r in top_jobs[:3]:
                print(f"\n  {r.job_title} at {r.company} ({r.score.overall_score}%)")
                for rec in r.score.recommendations[:2]:
                    print(f"    • {rec}")


def cmd_interactive(args):
    """Interactive job scoring mode."""
    # Load resume
    resume_text = Path(args.resume).read_text()
    profile = ResumeProfile(name="default", resume_text=resume_text)
    client = JobScanClient(mode=args.mode, debug=args.debug)

    print("JobScan Interactive Mode")
    print("Enter job details (or 'quit' to exit)")
    print("-" * 40)

    while True:
        try:
            print("\nJob Title (or 'quit'): ", end="")
            title = input().strip()
            if title.lower() == 'quit':
                break

            print("Company: ", end="")
            company = input().strip()

            print("Job URL (optional): ", end="")
            url = input().strip() or None

            print("Paste job description (end with empty line):")
            lines = []
            while True:
                line = input()
                if not line:
                    break
                lines.append(line)
            job_desc = "\n".join(lines)

            if not job_desc:
                print("No job description provided, skipping...")
                continue

            print("\nAnalyzing...")
            analysis = client.analyze_job(
                job_description=job_desc,
                job_title=title or "Unknown",
                company=company or "Unknown",
                job_url=url,
                resume=profile
            )

            print_score(analysis, verbose=True)

        except EOFError:
            break
        except KeyboardInterrupt:
            print("\nExiting...")
            break


def main():
    parser = argparse.ArgumentParser(
        description="Score job descriptions against your resume for ATS compatibility"
    )
    parser.add_argument(
        "--mode", "-m",
        choices=["auto", "api", "llm"],
        default="auto",
        help="Scoring mode: auto (detect), api (Apify), llm (local)"
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

    # Score single job
    score_parser = subparsers.add_parser("score", help="Score a single job")
    score_parser.add_argument(
        "--resume", "-r",
        required=True,
        help="Path to resume text file"
    )
    score_parser.add_argument(
        "job_description",
        nargs="?",
        help="Job description text (or use --job-file)"
    )
    score_parser.add_argument(
        "--job-file", "-f",
        help="Path to job description file"
    )
    score_parser.add_argument(
        "--title", "-t",
        help="Job title"
    )
    score_parser.add_argument(
        "--company", "-c",
        help="Company name"
    )
    score_parser.add_argument(
        "--url", "-u",
        help="Job posting URL"
    )
    score_parser.add_argument(
        "--no-cache",
        action="store_true",
        help="Skip cache"
    )
    score_parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Verbose output"
    )
    score_parser.set_defaults(func=cmd_score)

    # Batch scoring
    batch_parser = subparsers.add_parser("batch", help="Score multiple jobs")
    batch_parser.add_argument(
        "--resume", "-r",
        required=True,
        help="Path to resume text file"
    )
    batch_parser.add_argument(
        "file",
        help="JSON file with array of job objects"
    )
    batch_parser.set_defaults(func=cmd_batch)

    # Interactive mode
    interactive_parser = subparsers.add_parser("interactive", help="Interactive scoring")
    interactive_parser.add_argument(
        "--resume", "-r",
        required=True,
        help="Path to resume text file"
    )
    interactive_parser.set_defaults(func=cmd_interactive)

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    try:
        args.func(args)
    except FileNotFoundError as e:
        print(f"Error: File not found - {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        if args.debug:
            import traceback
            traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
