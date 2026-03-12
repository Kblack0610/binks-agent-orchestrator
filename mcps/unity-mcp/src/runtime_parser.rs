//! Multi-line block parser for Unity runtime console logs (Debug.Log/LogWarning/LogError).
//!
//! Unity runtime console output in Editor.log uses a multi-line block format:
//! ```text
//! <message text>
//! UnityEngine.Debug:ExtractStackTraceNoAlloc (byte*,int,string)
//! UnityEngine.StackTraceUtility:ExtractStackTrace () (at ...)
//! UnityEngine.DebugLogHandler:Internal_Log (...)
//! ...
//! UnityEngine.Debug:LogWarning (object)
//! CallingClass:Method (params) (at Assets/Scripts/file.cs:46)
//!
//! (Filename: Assets/Scripts/file.cs Line: 46)
//! ```
//!
//! The `(Filename: ... Line: N)` footer terminates each block. The log level is
//! determined by which `UnityEngine.Debug:LogXxx` call appears in the stack trace.

use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

/// Log level for runtime console entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeLogLevel {
    Log,
    Warning,
    Error,
    Exception,
}

impl RuntimeLogLevel {
    /// Check if this level matches a filter string.
    pub fn matches_filter(&self, filter: &str) -> bool {
        match filter.to_lowercase().as_str() {
            "log" => *self == RuntimeLogLevel::Log,
            "warning" | "warn" => *self == RuntimeLogLevel::Warning,
            "error" => *self == RuntimeLogLevel::Error,
            "exception" => *self == RuntimeLogLevel::Exception,
            "all" | "" => true,
            _ => true,
        }
    }
}

/// A single frame from a Unity runtime stack trace.
#[derive(Debug, Clone, Serialize)]
pub struct StackFrame {
    /// The class (possibly with namespace), e.g. "UnityEngine.Debug"
    pub namespace_class: String,
    /// The method name, e.g. "LogWarning"
    pub method: String,
    /// Source file path, if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Source line number, if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// A parsed runtime console log entry.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeLogEntry {
    /// The log message text (first line of the block)
    pub message: String,
    /// Detected log level
    pub level: RuntimeLogLevel,
    /// Stack trace frames (may be empty if `include_stack_traces` is false)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stack_trace: Vec<StackFrame>,
    /// Source file from the `(Filename: ...)` footer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    /// Source line from the `(Filename: ...)` footer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_line: Option<u32>,
    /// Line number in the log file where this entry starts
    pub line_number: usize,
    /// How many raw lines this entry spans
    pub raw_line_count: usize,
}

// ── Regex patterns ──────────────────────────────────────────────────────────

/// Matches the `(Filename: path Line: N)` footer that terminates a runtime log block.
static FILENAME_FOOTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\(Filename:\s*(.+?)\s+Line:\s*(\d+)\)\s*$").unwrap());

/// Matches a Unity-format stack frame line:
/// `Namespace.Class:Method (params) (at Assets/Scripts/file.cs:46)`
/// or without source info:
/// `UnityEngine.Debug:LogWarning (object)`
static UNITY_STACK_FRAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([\w][\w.<>]*(?:\.[\w][\w.<>]*)*):(\w+)\s*\(([^)]*)\)(?:\s*\(at\s+(.+?):(\d+)\))?\s*$",
    )
    .unwrap()
});

/// Matches exception-style first lines: `SomeException: message`
static RUNTIME_EXCEPTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\w+(?:\.\w+)*Exception):\s*(.*)$").unwrap());

// ── State machine ───────────────────────────────────────────────────────────

#[derive(Debug)]
enum ParserState {
    Idle,
    InBlock {
        message: String,
        start_line: usize,
        frames: Vec<StackFrame>,
        line_count: usize,
    },
}

/// Parse Unity runtime console log entries from raw text.
///
/// Uses a single-pass state machine:
/// - **Idle** → a non-stack-frame, non-footer line starts a new block as message
/// - **InBlock** → accumulate lines matching `UNITY_STACK_FRAME_RE`
/// - Footer (`FILENAME_FOOTER_RE`) finalizes the entry
///
/// `start_line` is the 1-based line number offset for the beginning of `text`.
pub fn parse_runtime_entries(text: &str, start_line: usize) -> Vec<RuntimeLogEntry> {
    let mut entries = Vec::new();
    let mut state = ParserState::Idle;
    let lines: Vec<&str> = text.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let current_line = start_line + i + 1;
        let trimmed = line.trim();

        // Skip blank lines
        if trimmed.is_empty() {
            // A blank line while in a block is part of the block (before footer)
            if let ParserState::InBlock { line_count, .. } = &mut state {
                *line_count += 1;
            }
            continue;
        }

        match &mut state {
            ParserState::Idle => {
                // Check if this is a footer without a block (shouldn't happen, skip)
                if FILENAME_FOOTER_RE.is_match(trimmed) {
                    continue;
                }
                // Check if this is a stack frame line without a message (orphaned, skip)
                if UNITY_STACK_FRAME_RE.is_match(trimmed) {
                    continue;
                }
                // Start a new block
                state = ParserState::InBlock {
                    message: trimmed.to_string(),
                    start_line: current_line,
                    frames: Vec::new(),
                    line_count: 1,
                };
            }
            ParserState::InBlock {
                message,
                start_line: block_start,
                frames,
                line_count,
            } => {
                // Check for footer → finalize entry
                if let Some(caps) = FILENAME_FOOTER_RE.captures(trimmed) {
                    *line_count += 1;
                    let source_file = caps.get(1).map(|m| m.as_str().to_string());
                    let source_line = caps
                        .get(2)
                        .and_then(|m| m.as_str().parse::<u32>().ok());

                    let level = determine_log_level(frames, message);

                    entries.push(RuntimeLogEntry {
                        message: message.clone(),
                        level,
                        stack_trace: std::mem::take(frames),
                        source_file,
                        source_line,
                        line_number: *block_start,
                        raw_line_count: *line_count,
                    });

                    state = ParserState::Idle;
                    continue;
                }

                // Check for stack frame
                if let Some(caps) = UNITY_STACK_FRAME_RE.captures(trimmed) {
                    *line_count += 1;
                    frames.push(StackFrame {
                        namespace_class: caps[1].to_string(),
                        method: caps[2].to_string(),
                        file: caps.get(4).map(|m| m.as_str().to_string()),
                        line: caps.get(5).and_then(|m| m.as_str().parse().ok()),
                    });
                    continue;
                }

                // Non-frame, non-footer line while in block → finalize current
                // block without footer (incomplete block) and start a new one
                let level = determine_log_level(frames, message);
                entries.push(RuntimeLogEntry {
                    message: message.clone(),
                    level,
                    stack_trace: std::mem::take(frames),
                    source_file: None,
                    source_line: None,
                    line_number: *block_start,
                    raw_line_count: *line_count,
                });

                // Check if new line is a footer or frame (unlikely, but handle)
                if FILENAME_FOOTER_RE.is_match(trimmed) || UNITY_STACK_FRAME_RE.is_match(trimmed) {
                    state = ParserState::Idle;
                } else {
                    state = ParserState::InBlock {
                        message: trimmed.to_string(),
                        start_line: current_line,
                        frames: Vec::new(),
                        line_count: 1,
                    };
                }
            }
        }
    }

    // Finalize any in-progress block at end of text
    if let ParserState::InBlock {
        message,
        start_line: block_start,
        mut frames,
        line_count,
    } = state
    {
        let level = determine_log_level(&frames, &message);
        entries.push(RuntimeLogEntry {
            message,
            level,
            stack_trace: std::mem::take(&mut frames),
            source_file: None,
            source_line: None,
            line_number: block_start,
            raw_line_count: line_count,
        });
    }

    entries
}

/// Determine the log level from stack frames and message text.
///
/// Priority:
/// 1. Stack contains `UnityEngine.Debug:LogError` → Error
/// 2. Stack contains `UnityEngine.Debug:LogWarning` → Warning
/// 3. Stack contains `UnityEngine.Debug:Log` → Log
/// 4. Message matches exception pattern → Exception
/// 5. Default → Log
fn determine_log_level(frames: &[StackFrame], message: &str) -> RuntimeLogLevel {
    for frame in frames {
        if frame.namespace_class == "UnityEngine.Debug" {
            match frame.method.as_str() {
                "LogError" | "LogException" => return RuntimeLogLevel::Error,
                "LogWarning" => return RuntimeLogLevel::Warning,
                "Log" => return RuntimeLogLevel::Log,
                _ => {}
            }
        }
    }

    if RUNTIME_EXCEPTION_RE.is_match(message) {
        return RuntimeLogLevel::Exception;
    }

    RuntimeLogLevel::Log
}

/// Check if a line looks like it's part of a Unity runtime log block
/// (either a stack frame or a filename footer).
pub fn is_runtime_log_line(line: &str) -> bool {
    let trimmed = line.trim();
    UNITY_STACK_FRAME_RE.is_match(trimmed) || FILENAME_FOOTER_RE.is_match(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_WARNING_BLOCK: &str = "\
this is a warning message
UnityEngine.Debug:ExtractStackTraceNoAlloc (byte*,int,string)
UnityEngine.StackTraceUtility:ExtractStackTrace ()
UnityEngine.DebugLogHandler:Internal_Log (UnityEngine.LogType,UnityEngine.LogOption,string,UnityEngine.Object)
UnityEngine.DebugLogHandler:LogFormat (UnityEngine.LogType,UnityEngine.Object,string,object[])
UnityEngine.Logger:Log (UnityEngine.LogType,object)
UnityEngine.Debug:LogWarning (object)
GameManager:Start () (at Assets/Scripts/GameManager.cs:46)

(Filename: Assets/Scripts/GameManager.cs Line: 46)";

    const SAMPLE_ERROR_BLOCK: &str = "\
Something went wrong!
UnityEngine.Debug:ExtractStackTraceNoAlloc (byte*,int,string)
UnityEngine.StackTraceUtility:ExtractStackTrace ()
UnityEngine.DebugLogHandler:Internal_Log (UnityEngine.LogType,UnityEngine.LogOption,string,UnityEngine.Object)
UnityEngine.DebugLogHandler:LogFormat (UnityEngine.LogType,UnityEngine.Object,string,object[])
UnityEngine.Logger:Log (UnityEngine.LogType,object)
UnityEngine.Debug:LogError (object)
PlayerController:Update () (at Assets/Scripts/PlayerController.cs:102)

(Filename: Assets/Scripts/PlayerController.cs Line: 102)";

    const SAMPLE_LOG_BLOCK: &str = "\
Player spawned at position (1, 2, 3)
UnityEngine.Debug:ExtractStackTraceNoAlloc (byte*,int,string)
UnityEngine.StackTraceUtility:ExtractStackTrace ()
UnityEngine.DebugLogHandler:Internal_Log (UnityEngine.LogType,UnityEngine.LogOption,string,UnityEngine.Object)
UnityEngine.DebugLogHandler:LogFormat (UnityEngine.LogType,UnityEngine.Object,string,object[])
UnityEngine.Logger:Log (UnityEngine.LogType,object)
UnityEngine.Debug:Log (object)
GameManager:SpawnPlayer () (at Assets/Scripts/GameManager.cs:80)

(Filename: Assets/Scripts/GameManager.cs Line: 80)";

    #[test]
    fn test_parse_warning_block() {
        let entries = parse_runtime_entries(SAMPLE_WARNING_BLOCK, 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "this is a warning message");
        assert_eq!(entries[0].level, RuntimeLogLevel::Warning);
        assert_eq!(
            entries[0].source_file.as_deref(),
            Some("Assets/Scripts/GameManager.cs")
        );
        assert_eq!(entries[0].source_line, Some(46));
        assert!(!entries[0].stack_trace.is_empty());
    }

    #[test]
    fn test_parse_error_block() {
        let entries = parse_runtime_entries(SAMPLE_ERROR_BLOCK, 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "Something went wrong!");
        assert_eq!(entries[0].level, RuntimeLogLevel::Error);
        assert_eq!(
            entries[0].source_file.as_deref(),
            Some("Assets/Scripts/PlayerController.cs")
        );
        assert_eq!(entries[0].source_line, Some(102));
    }

    #[test]
    fn test_parse_log_block() {
        let entries = parse_runtime_entries(SAMPLE_LOG_BLOCK, 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, RuntimeLogLevel::Log);
        assert_eq!(
            entries[0].message,
            "Player spawned at position (1, 2, 3)"
        );
    }

    #[test]
    fn test_parse_multiple_blocks() {
        let text = format!(
            "{}\n{}\n{}",
            SAMPLE_LOG_BLOCK, SAMPLE_WARNING_BLOCK, SAMPLE_ERROR_BLOCK
        );
        let entries = parse_runtime_entries(&text, 0);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].level, RuntimeLogLevel::Log);
        assert_eq!(entries[1].level, RuntimeLogLevel::Warning);
        assert_eq!(entries[2].level, RuntimeLogLevel::Error);
    }

    #[test]
    fn test_exception_detection() {
        let text = "\
NullReferenceException: Object reference not set to an instance of an object
UnityEngine.Debug:ExtractStackTraceNoAlloc (byte*,int,string)
UnityEngine.StackTraceUtility:ExtractStackTrace ()
PlayerController:Update () (at Assets/Scripts/PlayerController.cs:55)

(Filename: Assets/Scripts/PlayerController.cs Line: 55)";

        let entries = parse_runtime_entries(text, 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, RuntimeLogLevel::Exception);
        assert!(entries[0]
            .message
            .starts_with("NullReferenceException:"));
    }

    #[test]
    fn test_stack_frame_parsing() {
        let entries = parse_runtime_entries(SAMPLE_WARNING_BLOCK, 0);
        let frames = &entries[0].stack_trace;

        // Should have captured frames
        assert!(frames.len() >= 2);

        // Check the user code frame
        let user_frame = frames.iter().find(|f| f.namespace_class == "GameManager");
        assert!(user_frame.is_some());
        let uf = user_frame.unwrap();
        assert_eq!(uf.method, "Start");
        assert_eq!(uf.file.as_deref(), Some("Assets/Scripts/GameManager.cs"));
        assert_eq!(uf.line, Some(46));
    }

    #[test]
    fn test_line_numbers() {
        let text = format!("some editor line\n{}", SAMPLE_WARNING_BLOCK);
        let entries = parse_runtime_entries(&text, 0);
        // First entry is the editor line (not a runtime block, but parsed as a message)
        // Second entry is the warning block
        assert!(entries.len() >= 1);
    }

    #[test]
    fn test_empty_input() {
        let entries = parse_runtime_entries("", 0);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_is_runtime_log_line() {
        assert!(is_runtime_log_line(
            "UnityEngine.Debug:LogWarning (object)"
        ));
        assert!(is_runtime_log_line(
            "(Filename: Assets/Scripts/Foo.cs Line: 42)"
        ));
        assert!(!is_runtime_log_line("Just a regular log line"));
    }

    #[test]
    fn test_determine_log_level_from_frames() {
        let frames = vec![
            StackFrame {
                namespace_class: "UnityEngine.StackTraceUtility".to_string(),
                method: "ExtractStackTrace".to_string(),
                file: None,
                line: None,
            },
            StackFrame {
                namespace_class: "UnityEngine.Debug".to_string(),
                method: "LogWarning".to_string(),
                file: None,
                line: None,
            },
        ];
        assert_eq!(
            determine_log_level(&frames, "some message"),
            RuntimeLogLevel::Warning
        );
    }
}
