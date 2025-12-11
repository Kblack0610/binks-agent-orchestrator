"""
Memory Bank - File-based context management for long-running tasks.

The Memory Bank provides persistent project context through Markdown files.
Unlike Agno's SqliteAgentStorage (which handles agent state), the Memory Bank:
  - Lives in Git (human-readable, version-controlled)
  - Is editable by any tool (Claude, KiloCode, humans)
  - Survives across different agent sessions
  - Provides the "source of truth" for project state

Files managed:
  .orchestrator/
  ├── productContext.md    # The "Why" - high-level goals, success criteria
  ├── activeContext.md     # The "What" - current state, progress, next steps
  └── systemPatterns.md    # Architectural decisions, code patterns, lessons
"""
from pathlib import Path
from typing import Optional
from datetime import datetime


class MemoryBank:
    """
    File-based context management for long-running tasks.

    Usage:
        bank = MemoryBank()
        bank.initialize("Build a REST API for user management")

        # In agent prompts
        context = bank.read_context()

        # After each step
        bank.update_active_context("## Current State\\n- Completed: API design...")
    """

    def __init__(self, base_dir: Optional[Path] = None):
        """
        Initialize the Memory Bank.

        Args:
            base_dir: Directory for memory bank files (default: .orchestrator)
        """
        self.base_dir = Path(base_dir) if base_dir else Path(".orchestrator")

    @property
    def product_context(self) -> Path:
        """Path to product context file (the "Why")."""
        return self.base_dir / "productContext.md"

    @property
    def active_context(self) -> Path:
        """Path to active context file (the "What")."""
        return self.base_dir / "activeContext.md"

    @property
    def system_patterns(self) -> Path:
        """Path to system patterns file (architectural decisions)."""
        return self.base_dir / "systemPatterns.md"

    def exists(self) -> bool:
        """Check if memory bank has been initialized."""
        return self.active_context.exists()

    def initialize(self, goal: str, project_info: str = "") -> None:
        """
        Initialize memory bank for a new task.

        Args:
            goal: The high-level goal for this project
            project_info: Additional project context
        """
        self.base_dir.mkdir(parents=True, exist_ok=True)

        # Product Context - The "Why"
        self.product_context.write_text(f"""# Product Context

## Goal
{goal}

## Project Info
{project_info}

## Success Criteria
- [ ] Define during planning phase

## Constraints
- [ ] Define any constraints or requirements
""")

        # Active Context - The "What"
        self.active_context.write_text(f"""# Active Context
_Last updated: {datetime.now().isoformat()}_

## Current State
- Status: INITIALIZED
- Phase: PLANNING
- Iteration: 0

## What We're Working On
(To be updated by agents)

## Recent Progress
(Empty - just started)

## Blockers
(None yet)

## Next Steps
1. Generate implementation plan
""")

        # System Patterns - Architectural decisions
        self.system_patterns.write_text("""# System Patterns

## Architectural Decisions
(To be populated during design phase)

## Code Patterns
(To be populated during implementation)

## API Contracts
(Define interfaces between components)

## Lessons Learned
(Updated after each iteration)
""")

    def read_context(self, max_chars: int = 50000) -> str:
        """
        Read all context files into a single prompt-ready string.

        Args:
            max_chars: Maximum characters to return (triggers compaction warning)

        Returns:
            Combined content of all memory bank files
        """
        parts = []

        if self.product_context.exists():
            parts.append(f"=== PRODUCT CONTEXT ===\n{self.product_context.read_text()}")

        if self.active_context.exists():
            parts.append(f"=== ACTIVE CONTEXT ===\n{self.active_context.read_text()}")

        if self.system_patterns.exists():
            parts.append(f"=== SYSTEM PATTERNS ===\n{self.system_patterns.read_text()}")

        content = "\n\n".join(parts)

        if len(content) > max_chars:
            content = f"[WARNING: Context exceeds {max_chars} chars. Consider compacting.]\n\n{content}"

        return content

    def read_active_context(self) -> str:
        """Read just the active context file."""
        if self.active_context.exists():
            return self.active_context.read_text()
        return ""

    def update_active_context(self, content: str) -> None:
        """
        Update active context with new state.

        Args:
            content: New content (header is added automatically)
        """
        self.base_dir.mkdir(parents=True, exist_ok=True)
        header = f"# Active Context\n_Last updated: {datetime.now().isoformat()}_\n\n"
        self.active_context.write_text(header + content)

    def append_to_active_context(self, section: str, content: str) -> None:
        """
        Append content to a section in active context.

        Args:
            section: Section header (e.g., "Recent Progress")
            content: Content to append
        """
        current = self.read_active_context()

        # Find the section and append
        if f"## {section}" in current:
            # Insert before the next section
            lines = current.split("\n")
            new_lines = []
            in_section = False

            for line in lines:
                if line.startswith(f"## {section}"):
                    in_section = True
                    new_lines.append(line)
                    continue

                if in_section and line.startswith("## "):
                    # Found next section, insert content before it
                    new_lines.append(content)
                    new_lines.append("")
                    in_section = False

                new_lines.append(line)

            if in_section:
                # Section was at the end
                new_lines.append(content)

            self.active_context.write_text("\n".join(new_lines))
        else:
            # Section doesn't exist, append at end
            current += f"\n\n## {section}\n{content}"
            self.active_context.write_text(current)

    def append_pattern(self, pattern_name: str, pattern_content: str) -> None:
        """
        Add a new pattern to system patterns.

        Args:
            pattern_name: Name/title of the pattern
            pattern_content: Description of the pattern
        """
        self.base_dir.mkdir(parents=True, exist_ok=True)

        current = ""
        if self.system_patterns.exists():
            current = self.system_patterns.read_text()

        new_content = f"{current}\n\n## {pattern_name}\n{pattern_content}"
        self.system_patterns.write_text(new_content)

    def update_status(self, status: str, phase: str, iteration: int = None) -> None:
        """
        Update the status section of active context.

        Args:
            status: Current status (PLANNING, CODING, REVIEWING, etc.)
            phase: Current phase description
            iteration: Iteration count (optional)
        """
        current = self.read_active_context()

        # Build new status section
        status_content = f"""## Current State
- Status: {status}
- Phase: {phase}"""

        if iteration is not None:
            status_content += f"\n- Iteration: {iteration}"

        # Replace existing status section
        import re
        pattern = r"## Current State\n.*?(?=\n## |\Z)"
        if re.search(pattern, current, re.DOTALL):
            current = re.sub(pattern, status_content + "\n", current, flags=re.DOTALL)
        else:
            # No status section, add at beginning after header
            lines = current.split("\n")
            for i, line in enumerate(lines):
                if line.startswith("## "):
                    lines.insert(i, status_content + "\n")
                    break
            current = "\n".join(lines)

        self.active_context.write_text(current)

    def compact(self, summarizer_callable=None) -> str:
        """
        Compact the memory bank by summarizing context.

        Args:
            summarizer_callable: Function that takes text and returns summary.
                                 If None, returns content for manual summarization.

        Returns:
            Summary text or instruction to summarize manually
        """
        context = self.read_context(max_chars=999999)

        if summarizer_callable:
            prompt = f"""Summarize this context, preserving key decisions and current state:

{context}

Provide a condensed version that captures:
1. Core goal and success criteria
2. Key architectural decisions
3. Current progress and blockers
4. Immediate next steps"""

            summary = summarizer_callable(prompt)
            self.update_active_context(summary)
            return "Context compacted."

        return f"Context is {len(context)} chars. Provide a summarizer to compact."

    def clear(self) -> None:
        """Clear all memory bank files."""
        for filepath in [self.product_context, self.active_context, self.system_patterns]:
            if filepath.exists():
                filepath.unlink()

        if self.base_dir.exists() and not any(self.base_dir.iterdir()):
            self.base_dir.rmdir()


# Standalone usage example
if __name__ == "__main__":
    bank = MemoryBank()

    print("Initializing memory bank...")
    bank.initialize(
        goal="Test the MemoryBank class",
        project_info="CLI Orchestrator development"
    )

    print("\nReading context:")
    print(bank.read_context()[:500] + "...")

    print("\nUpdating status...")
    bank.update_status("CODING", "Implementing core features", iteration=1)

    print("\nAppending pattern...")
    bank.append_pattern(
        "Hybrid Persistence",
        "Use Agno SqliteAgentStorage for agent state, Memory Bank for project context."
    )

    print("\nFinal context:")
    print(bank.read_context())
