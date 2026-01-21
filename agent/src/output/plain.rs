//! Plain text output for pipes and CI environments
//!
//! No colors or special formatting - just clean text output.

use std::io::{self, Write};

use super::{OutputEvent, OutputWriter};

/// Plain text output writer (no colors)
pub struct PlainOutput {
    /// Whether to show verbose output
    verbose: bool,
}

impl Default for PlainOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl PlainOutput {
    /// Create a new plain output writer
    pub fn new() -> Self {
        Self { verbose: false }
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Format tool arguments for display
    fn format_args(&self, args: &serde_json::Value) -> String {
        match args {
            serde_json::Value::Object(map) if map.is_empty() => String::new(),
            serde_json::Value::Null => String::new(),
            _ => {
                let json = serde_json::to_string(args).unwrap_or_default();
                if json.len() > 80 {
                    format!("{}...", &json[..77])
                } else {
                    json
                }
            }
        }
    }
}

impl OutputWriter for PlainOutput {
    fn write(&self, event: OutputEvent) {
        match event {
            OutputEvent::Text(text) => {
                println!("{}", text);
            }

            OutputEvent::ToolStart { name, arguments } => {
                let args_str = self.format_args(&arguments);
                if args_str.is_empty() {
                    eprintln!("  -> {}", name);
                } else {
                    eprintln!("  -> {} {}", name, args_str);
                }
            }

            OutputEvent::ToolComplete {
                name,
                result,
                duration,
                is_error,
            } => {
                let status = if is_error { "FAIL" } else { "OK" };
                let time = format!("({}ms)", duration.as_millis());

                if self.verbose || is_error {
                    let preview = if result.len() > 100 {
                        format!("{}...", &result[..97])
                    } else {
                        result
                    };
                    eprintln!("  {} {} {} {}", status, name, time, preview);
                } else {
                    eprintln!("  {} {} {}", status, name, time);
                }
            }

            OutputEvent::Thinking(content) => {
                eprintln!("  [thinking] {}", content);
            }

            OutputEvent::Token(token) => {
                print!("{}", token);
                let _ = io::stdout().flush();
            }

            OutputEvent::Progress { message, done } => {
                let status = if done { "DONE" } else { "..." };
                eprintln!("  {} {}", status, message);
            }

            OutputEvent::Status(msg) => {
                eprintln!("  {}", msg);
            }

            OutputEvent::Error(msg) => {
                eprintln!("Error: {}", msg);
            }

            OutputEvent::Warning(msg) => {
                eprintln!("Warning: {}", msg);
            }

            OutputEvent::System(msg) => {
                eprintln!("{}", msg);
            }

            OutputEvent::NewLine => {
                println!();
            }
        }
    }

    fn flush(&self) {
        let _ = io::stdout().flush();
        let _ = io::stderr().flush();
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_colors(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_output_creation() {
        let output = PlainOutput::new();
        assert!(!output.verbose);

        let output = PlainOutput::new().with_verbose(true);
        assert!(output.verbose);
    }

    #[test]
    fn test_format_args() {
        let output = PlainOutput::new();

        let empty = serde_json::json!({});
        assert_eq!(output.format_args(&empty), "");

        let args = serde_json::json!({"key": "value"});
        let formatted = output.format_args(&args);
        assert!(formatted.contains("key"));
    }

    #[test]
    fn test_no_colors() {
        let output = PlainOutput::new();
        assert!(!output.supports_colors());
        assert!(output.supports_streaming());
    }
}
