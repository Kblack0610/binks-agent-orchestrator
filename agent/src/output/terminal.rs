//! Terminal output with colors and formatting
//!
//! Uses ANSI escape codes for colors and styling.

use std::io::{self, Write};

use super::{OutputEvent, OutputWriter};

// ANSI color codes
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const ITALIC: &str = "\x1b[3m";

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const GRAY: &str = "\x1b[90m";

/// Terminal output writer with colors and formatting
pub struct TerminalOutput {
    /// Whether to use colors (can be disabled)
    use_colors: bool,
    /// Whether verbose mode is enabled
    verbose: bool,
}

impl Default for TerminalOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalOutput {
    /// Create a new terminal output writer
    pub fn new() -> Self {
        Self {
            use_colors: true,
            verbose: false,
        }
    }

    /// Create with auto-detected settings (enables colors if stdout is a TTY)
    pub fn auto() -> Self {
        use std::io::IsTerminal;
        Self {
            use_colors: std::io::stdout().is_terminal(),
            verbose: false,
        }
    }

    /// Create without colors
    pub fn without_colors() -> Self {
        Self {
            use_colors: false,
            verbose: false,
        }
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Format with color if colors are enabled
    fn color(&self, code: &str, text: &str) -> String {
        if self.use_colors {
            format!("{}{}{}", code, text, RESET)
        } else {
            text.to_string()
        }
    }

    /// Format with multiple styles
    fn styled(&self, codes: &[&str], text: &str) -> String {
        if self.use_colors {
            let prefix: String = codes.iter().copied().collect();
            format!("{}{}{}", prefix, text, RESET)
        } else {
            text.to_string()
        }
    }

    /// Print to stderr (for status/progress messages)
    fn eprint(&self, msg: &str) {
        eprintln!("{}", msg);
    }

    /// Print to stdout (for content)
    fn print(&self, msg: &str) {
        println!("{}", msg);
    }

    /// Print without newline (for streaming)
    fn print_inline(&self, msg: &str) {
        print!("{}", msg);
        let _ = io::stdout().flush();
    }

    /// Format tool arguments for display
    fn format_args(&self, args: &serde_json::Value) -> String {
        match args {
            serde_json::Value::Object(map) if map.is_empty() => String::new(),
            serde_json::Value::Null => String::new(),
            _ => {
                let json = serde_json::to_string(args).unwrap_or_default();
                // Truncate long arguments
                if json.len() > 80 {
                    format!("{}...", &json[..77])
                } else {
                    json
                }
            }
        }
    }
}

impl OutputWriter for TerminalOutput {
    fn write(&self, event: OutputEvent) {
        match event {
            OutputEvent::Text(text) => {
                self.print(&text);
            }

            OutputEvent::ToolStart { name, arguments } => {
                let tool_name = self.styled(&[BOLD, CYAN], &name);
                let args_str = self.format_args(&arguments);

                if args_str.is_empty() {
                    self.eprint(&format!("  {} {}", self.color(GRAY, "→"), tool_name));
                } else {
                    let args_display = self.color(GRAY, &args_str);
                    self.eprint(&format!(
                        "  {} {} {}",
                        self.color(GRAY, "→"),
                        tool_name,
                        args_display
                    ));
                }
            }

            OutputEvent::ToolComplete {
                name,
                result,
                duration,
                is_error,
            } => {
                let status = if is_error {
                    self.color(RED, "✗")
                } else {
                    self.color(GREEN, "✓")
                };

                let time = self.color(GRAY, &format!("({:.0}ms)", duration.as_millis()));

                // Show result preview in verbose mode or on error
                if self.verbose || is_error {
                    let preview = if result.len() > 100 {
                        format!("{}...", &result[..97])
                    } else {
                        result
                    };
                    let preview_styled = if is_error {
                        self.color(RED, &preview)
                    } else {
                        self.color(GRAY, &preview)
                    };
                    self.eprint(&format!(
                        "  {} {} {} {}",
                        status, name, time, preview_styled
                    ));
                } else {
                    self.eprint(&format!("  {} {} {}", status, name, time));
                }
            }

            OutputEvent::Thinking(content) => {
                // Display thinking in dim italic
                let styled = self.styled(&[DIM, ITALIC], &content);
                self.eprint(&format!("  {} {}", self.color(MAGENTA, "💭"), styled));
            }

            OutputEvent::Token(token) => {
                // Stream tokens inline without newline
                self.print_inline(&token);
            }

            OutputEvent::Progress { message, done } => {
                if done {
                    self.eprint(&format!(
                        "  {} {}",
                        self.color(GREEN, "✓"),
                        self.color(GRAY, &message)
                    ));
                } else {
                    self.eprint(&format!(
                        "  {} {}",
                        self.color(BLUE, "⋯"),
                        self.color(GRAY, &message)
                    ));
                }
            }

            OutputEvent::Status(msg) => {
                self.eprint(&self.color(GRAY, &format!("  {}", msg)));
            }

            OutputEvent::Error(msg) => {
                self.eprint(&format!(
                    "{} {}",
                    self.styled(&[BOLD, RED], "Error:"),
                    self.color(RED, &msg)
                ));
            }

            OutputEvent::Warning(msg) => {
                self.eprint(&format!(
                    "{} {}",
                    self.styled(&[BOLD, YELLOW], "Warning:"),
                    self.color(YELLOW, &msg)
                ));
            }

            OutputEvent::System(msg) => {
                self.eprint(&self.color(GRAY, &msg));
            }

            OutputEvent::NewLine => {
                self.print("");
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
        self.use_colors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_output_creation() {
        let output = TerminalOutput::new();
        assert!(output.use_colors);
        assert!(!output.verbose);

        let output = TerminalOutput::without_colors();
        assert!(!output.use_colors);
    }

    #[test]
    fn test_color_formatting() {
        let output = TerminalOutput::new();
        let colored = output.color(RED, "test");
        assert!(colored.contains("\x1b[31m"));
        assert!(colored.contains("\x1b[0m"));
        assert!(colored.contains("test"));

        let output = TerminalOutput::without_colors();
        let plain = output.color(RED, "test");
        assert_eq!(plain, "test");
    }

    #[test]
    fn test_format_args() {
        let output = TerminalOutput::new();

        // Empty object
        let empty = serde_json::json!({});
        assert_eq!(output.format_args(&empty), "");

        // Null
        let null = serde_json::Value::Null;
        assert_eq!(output.format_args(&null), "");

        // Normal object
        let args = serde_json::json!({"key": "value"});
        let formatted = output.format_args(&args);
        assert!(formatted.contains("key"));
        assert!(formatted.contains("value"));
    }

    #[test]
    fn test_supports_streaming() {
        let output = TerminalOutput::new();
        assert!(output.supports_streaming());
        assert!(output.supports_colors());
    }
}
