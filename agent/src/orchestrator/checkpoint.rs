//! Checkpoint handling for human-in-loop approval
//!
//! Provides interactive prompts for workflow checkpoints
//! where human approval is required before proceeding.

use std::io::{self, BufRead, Write};

/// Result of a checkpoint interaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointResult {
    /// User approved, continue workflow
    Approved,
    /// User approved with modifications
    ApprovedWithNote(String),
    /// User rejected, stop workflow
    Rejected,
    /// User wants to edit before proceeding
    Edit(String),
}

/// A checkpoint that pauses workflow for user input
pub struct Checkpoint {
    /// Message to display to the user
    pub message: String,
    /// Content to show before the prompt (e.g., the plan)
    pub content: Option<String>,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            content: None,
        }
    }

    /// Set content to display before the prompt
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Execute the checkpoint interactively
    ///
    /// This will:
    /// 1. Display the content (if any)
    /// 2. Display the message
    /// 3. Prompt for user input
    /// 4. Return the result
    pub fn execute(&self) -> io::Result<CheckpointResult> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        // Display separator
        println!("\n{}", "═".repeat(60));
        println!("  CHECKPOINT");
        println!("{}\n", "═".repeat(60));

        // Display content if present
        if let Some(content) = &self.content {
            println!("{}\n", content);
            println!("{}", "─".repeat(60));
        }

        // Display message
        println!("\n{}\n", self.message);

        // Show options
        println!("Options:");
        println!("  [y/yes]     - Approve and continue");
        println!("  [n/no]      - Reject and stop workflow");
        println!("  [e/edit]    - Provide modifications");
        println!("  [note TEXT] - Approve with a note");
        println!();

        // Prompt for input
        print!("Your choice: ");
        stdout.flush()?;

        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        let result = match input.as_str() {
            "y" | "yes" | "" => CheckpointResult::Approved,
            "n" | "no" => CheckpointResult::Rejected,
            "e" | "edit" => {
                println!("\nEnter your modifications (end with empty line):");
                let mut edits = Vec::new();
                loop {
                    let mut line = String::new();
                    stdin.lock().read_line(&mut line)?;
                    if line.trim().is_empty() {
                        break;
                    }
                    edits.push(line);
                }
                CheckpointResult::Edit(edits.join(""))
            }
            s if s.starts_with("note ") => CheckpointResult::ApprovedWithNote(s[5..].to_string()),
            _ => {
                println!("Invalid input, treating as rejection for safety.");
                CheckpointResult::Rejected
            }
        };

        println!("\n{}", "═".repeat(60));

        Ok(result)
    }

    /// Execute checkpoint in non-interactive mode (auto-approve)
    pub fn execute_auto(&self) -> CheckpointResult {
        if let Some(content) = &self.content {
            println!("\n{}", content);
        }
        println!(
            "\n[Checkpoint: {}] Auto-approved (non-interactive mode)",
            self.message
        );
        CheckpointResult::Approved
    }
}

/// Trait for checkpoint handling strategies
pub trait CheckpointHandler: Send + Sync {
    /// Handle a checkpoint and return the result
    fn handle(&self, checkpoint: &Checkpoint) -> CheckpointResult;
}

/// Interactive checkpoint handler (default)
pub struct InteractiveCheckpointHandler;

impl CheckpointHandler for InteractiveCheckpointHandler {
    fn handle(&self, checkpoint: &Checkpoint) -> CheckpointResult {
        checkpoint.execute().unwrap_or(CheckpointResult::Rejected)
    }
}

/// Auto-approve checkpoint handler (for testing/CI)
pub struct AutoApproveCheckpointHandler;

impl CheckpointHandler for AutoApproveCheckpointHandler {
    fn handle(&self, checkpoint: &Checkpoint) -> CheckpointResult {
        checkpoint.execute_auto()
    }
}

/// Always reject checkpoint handler (for testing)
pub struct RejectCheckpointHandler;

impl CheckpointHandler for RejectCheckpointHandler {
    fn handle(&self, checkpoint: &Checkpoint) -> CheckpointResult {
        println!(
            "\n[Checkpoint: {}] Auto-rejected (testing mode)",
            checkpoint.message
        );
        CheckpointResult::Rejected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_creation() {
        let checkpoint = Checkpoint::new("Approve plan?").with_content("Here is the plan...");

        assert_eq!(checkpoint.message, "Approve plan?");
        assert_eq!(checkpoint.content, Some("Here is the plan...".to_string()));
    }

    #[test]
    fn test_auto_approve() {
        let checkpoint = Checkpoint::new("Test checkpoint");
        let result = checkpoint.execute_auto();
        assert_eq!(result, CheckpointResult::Approved);
    }

    #[test]
    fn test_handler_auto_approve() {
        let handler = AutoApproveCheckpointHandler;
        let checkpoint = Checkpoint::new("Test");
        let result = handler.handle(&checkpoint);
        assert_eq!(result, CheckpointResult::Approved);
    }

    #[test]
    fn test_handler_reject() {
        let handler = RejectCheckpointHandler;
        let checkpoint = Checkpoint::new("Test");
        let result = handler.handle(&checkpoint);
        assert_eq!(result, CheckpointResult::Rejected);
    }
}
