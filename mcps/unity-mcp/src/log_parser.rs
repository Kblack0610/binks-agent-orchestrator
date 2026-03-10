//! Regex-based Unity Editor.log line parsing

use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

/// A parsed log entry from Editor.log.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub line_number: usize,
    pub level: LogLevel,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Log severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warning,
    Info,
}

impl LogLevel {
    pub fn matches_filter(&self, filter: &str) -> bool {
        match filter.to_lowercase().as_str() {
            "error" => *self == LogLevel::Error,
            "warning" | "warn" => *self == LogLevel::Warning,
            "info" => *self == LogLevel::Info,
            _ => true,
        }
    }
}

/// A structured compile error extracted from the log.
#[derive(Debug, Clone, Serialize)]
pub struct CompileError {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub code: String,
    pub message: String,
}

/// An exception extracted from the log.
#[derive(Debug, Clone, Serialize)]
pub struct ExceptionEntry {
    pub exception_type: String,
    pub message: String,
    pub stack_trace: Vec<String>,
}

// Precompiled regex patterns
static TIMESTAMP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[(\d{2}:\d{2}:\d{2}\.\d+)\]\[(\d+)\]\s*(.*)").unwrap());

static COMPILE_ERROR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(Assets/[^\(]+)\((\d+),(\d+)\):\s*(error\s+\w+):\s*(.+)$").unwrap()
});

static COMPILE_WARNING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(Assets/[^\(]+)\((\d+),(\d+)\):\s*(warning\s+\w+):\s*(.+)$").unwrap()
});

static EXCEPTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\w+(?:\.\w+)*Exception):\s*(.+)$").unwrap()
});

/// Classify a log line into a severity level.
pub fn classify_line(line: &str) -> LogLevel {
    let text = strip_timestamp(line);

    if COMPILE_ERROR_RE.is_match(text)
        || text.starts_with("Error")
        || text.contains("error CS")
        || text.contains("NullReferenceException")
        || text.contains("Exception:")
        || text.starts_with("Debug.LogError")
    {
        return LogLevel::Error;
    }

    if COMPILE_WARNING_RE.is_match(text)
        || text.starts_with("Warning")
        || text.starts_with("Debug.LogWarning")
        || text.contains("warning CS")
    {
        return LogLevel::Warning;
    }

    LogLevel::Info
}

/// Strip the timestamp prefix if present, returning the message portion.
fn strip_timestamp(line: &str) -> &str {
    if let Some(caps) = TIMESTAMP_RE.captures(line) {
        caps.get(3).map_or(line, |m| m.as_str())
    } else {
        line
    }
}

/// Extract timestamp from a line if present.
fn extract_timestamp(line: &str) -> Option<String> {
    TIMESTAMP_RE
        .captures(line)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Parse raw log text into structured entries.
pub fn parse_lines(text: &str, start_line: usize) -> Vec<LogEntry> {
    text.lines()
        .enumerate()
        .map(|(i, line)| LogEntry {
            line_number: start_line + i + 1,
            level: classify_line(line),
            message: line.to_string(),
            timestamp: extract_timestamp(line),
        })
        .collect()
}

/// Extract compile errors from log text.
pub fn extract_compile_errors(text: &str) -> Vec<CompileError> {
    text.lines()
        .filter_map(|line| {
            let text = strip_timestamp(line);
            COMPILE_ERROR_RE.captures(text).map(|caps| CompileError {
                file: caps[1].to_string(),
                line: caps[2].parse().unwrap_or(0),
                column: caps[3].parse().unwrap_or(0),
                code: caps[4].to_string(),
                message: caps[5].to_string(),
            })
        })
        .collect()
}

/// Extract exceptions from log text.
///
/// Collects exception lines and subsequent indented stack trace lines.
pub fn extract_exceptions(text: &str) -> Vec<ExceptionEntry> {
    let mut exceptions = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = strip_timestamp(lines[i]);
        if let Some(caps) = EXCEPTION_RE.captures(line) {
            let mut entry = ExceptionEntry {
                exception_type: caps[1].to_string(),
                message: caps[2].to_string(),
                stack_trace: Vec::new(),
            };

            // Collect indented stack trace lines
            i += 1;
            while i < lines.len() {
                let next = strip_timestamp(lines[i]);
                if next.starts_with("  at ") || next.starts_with("   at ") {
                    entry.stack_trace.push(next.trim().to_string());
                    i += 1;
                } else {
                    break;
                }
            }

            exceptions.push(entry);
        } else {
            i += 1;
        }
    }

    exceptions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_error() {
        assert_eq!(
            classify_line("Assets/Scripts/Foo.cs(42,10): error CS1002: ; expected"),
            LogLevel::Error
        );
    }

    #[test]
    fn test_classify_warning() {
        assert_eq!(
            classify_line("Assets/Scripts/Bar.cs(10,5): warning CS0168: unused variable"),
            LogLevel::Warning
        );
    }

    #[test]
    fn test_classify_info() {
        assert_eq!(
            classify_line("Begin MonoManager ReloadAssembly"),
            LogLevel::Info
        );
    }

    #[test]
    fn test_classify_exception() {
        assert_eq!(
            classify_line("NullReferenceException: Object reference not set"),
            LogLevel::Error
        );
    }

    #[test]
    fn test_extract_timestamp() {
        let ts = extract_timestamp("[14:30:22.123][1] Some message");
        assert_eq!(ts, Some("14:30:22.123".to_string()));
    }

    #[test]
    fn test_no_timestamp() {
        let ts = extract_timestamp("Just a regular line");
        assert_eq!(ts, None);
    }

    #[test]
    fn test_extract_compile_errors() {
        let log = "\
Begin MonoManager ReloadAssembly
Assets/Scripts/Foo.cs(42,10): error CS1002: ; expected
Assets/Scripts/Bar.cs(7,1): error CS0246: type not found
Refresh completed in 0.5 seconds";

        let errors = extract_compile_errors(log);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].file, "Assets/Scripts/Foo.cs");
        assert_eq!(errors[0].line, 42);
        assert_eq!(errors[0].column, 10);
        assert_eq!(errors[0].code, "error CS1002");
        assert_eq!(errors[0].message, "; expected");
        assert_eq!(errors[1].file, "Assets/Scripts/Bar.cs");
    }

    #[test]
    fn test_extract_exceptions() {
        let log = "\
Some info line
NullReferenceException: Object reference not set
  at MyClass.Update () [0x00000] in Assets/Scripts/MyClass.cs:15
  at UnityEngine.Component.SendMessage ()
Another info line";

        let exceptions = extract_exceptions(log);
        assert_eq!(exceptions.len(), 1);
        assert_eq!(exceptions[0].exception_type, "NullReferenceException");
        assert_eq!(exceptions[0].stack_trace.len(), 2);
    }

    #[test]
    fn test_parse_lines() {
        let log = "Info line\nAssets/Foo.cs(1,1): error CS0001: bad\nWarning line";
        let entries = parse_lines(log, 0);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].level, LogLevel::Info);
        assert_eq!(entries[1].level, LogLevel::Error);
        assert_eq!(entries[1].line_number, 2);
    }

    #[test]
    fn test_timestamped_error() {
        assert_eq!(
            classify_line("[14:30:22.123][1] Assets/Scripts/Foo.cs(42,10): error CS1002: ; expected"),
            LogLevel::Error
        );
    }
}
