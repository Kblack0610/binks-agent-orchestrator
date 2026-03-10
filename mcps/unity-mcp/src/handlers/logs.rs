//! Log reading, tailing, and error parsing handlers

use crate::detect;
use crate::log_parser::{self, LogEntry};
use mcp_common::McpError;
use regex::Regex;
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

/// Response for unity_read_log
#[derive(Serialize)]
pub struct ReadLogResponse {
    pub log_path: String,
    pub total_lines: usize,
    pub returned: usize,
    pub entries: Vec<LogEntry>,
}

/// Response for unity_log_errors
#[derive(Serialize)]
pub struct LogErrorsResponse {
    pub log_path: String,
    pub compile_errors: Vec<log_parser::CompileError>,
    pub exceptions: Vec<log_parser::ExceptionEntry>,
    pub total_errors: usize,
}

/// Response for unity_log_tail
#[derive(Serialize)]
pub struct LogTailResponse {
    pub log_path: String,
    pub new_bytes: u64,
    pub entries: Vec<LogEntry>,
    pub offset: u64,
}

/// Resolve the log path from an optional override or auto-detection.
fn resolve_log_path(override_path: Option<&str>) -> Result<PathBuf, McpError> {
    if let Some(p) = override_path {
        let path = PathBuf::from(p);
        if path.exists() {
            return Ok(path);
        }
        return Err(McpError::invalid_params(
            format!("Log file not found: {}", p),
            None,
        ));
    }

    detect::find_log_path().ok_or_else(|| {
        McpError::internal_error(
            "Could not find Unity Editor.log. Set UNITY_LOG_PATH env var or pass log_path parameter."
                .to_string(),
            None,
        )
    })
}

/// Read the last N lines of the log, optionally filtered by level/pattern.
pub fn read_log(
    lines: usize,
    level: Option<&str>,
    pattern: Option<&str>,
    log_path: Option<&str>,
) -> Result<ReadLogResponse, McpError> {
    let path = resolve_log_path(log_path)?;
    let content = std::fs::read_to_string(&path).map_err(|e| {
        McpError::internal_error(format!("Failed to read log file: {}", e), None)
    })?;

    let all_lines: Vec<&str> = content.lines().collect();
    let total_lines = all_lines.len();

    let pattern_re = pattern
        .map(|p| {
            Regex::new(p).map_err(|e| {
                McpError::invalid_params(format!("Invalid regex pattern: {}", e), None)
            })
        })
        .transpose()?;

    let mut entries = log_parser::parse_lines(&content, 0);

    // Apply filters
    if let Some(level_filter) = level {
        entries.retain(|e| e.level.matches_filter(level_filter));
    }

    if let Some(re) = &pattern_re {
        entries.retain(|e| re.is_match(&e.message));
    }

    // Take last N
    let returned = entries.len().min(lines);
    if entries.len() > lines {
        entries = entries.split_off(entries.len() - lines);
    }

    Ok(ReadLogResponse {
        log_path: path.display().to_string(),
        total_lines,
        returned,
        entries,
    })
}

/// Parse the log for compile errors and exceptions.
pub fn log_errors(log_path: Option<&str>) -> Result<LogErrorsResponse, McpError> {
    let path = resolve_log_path(log_path)?;
    let content = std::fs::read_to_string(&path).map_err(|e| {
        McpError::internal_error(format!("Failed to read log file: {}", e), None)
    })?;

    let compile_errors = log_parser::extract_compile_errors(&content);
    let exceptions = log_parser::extract_exceptions(&content);
    let total_errors = compile_errors.len() + exceptions.len();

    Ok(LogErrorsResponse {
        log_path: path.display().to_string(),
        compile_errors,
        exceptions,
        total_errors,
    })
}

/// Read new log entries since the last call (stateful via offset).
///
/// Returns new entries and the updated offset. Handles log rotation
/// by resetting to 0 if the file shrinks.
pub fn log_tail(
    current_offset: u64,
    log_path: Option<&str>,
) -> Result<(LogTailResponse, u64), McpError> {
    let path = resolve_log_path(log_path)?;

    let mut file = std::fs::File::open(&path).map_err(|e| {
        McpError::internal_error(format!("Failed to open log file: {}", e), None)
    })?;

    let metadata = file.metadata().map_err(|e| {
        McpError::internal_error(format!("Failed to read file metadata: {}", e), None)
    })?;

    let file_len = metadata.len();

    // Handle log rotation: if file is smaller than our offset, start from 0
    let seek_pos = if file_len < current_offset {
        0
    } else {
        current_offset
    };

    file.seek(SeekFrom::Start(seek_pos)).map_err(|e| {
        McpError::internal_error(format!("Failed to seek in log file: {}", e), None)
    })?;

    let mut new_content = String::new();
    file.read_to_string(&mut new_content).map_err(|e| {
        McpError::internal_error(format!("Failed to read new log content: {}", e), None)
    })?;

    let new_bytes = file_len.saturating_sub(seek_pos);
    let line_offset = if seek_pos == 0 {
        0
    } else {
        // Approximate line number from previous content
        // This is best-effort; exact line tracking would require reading the whole file
        0
    };

    let entries = if new_content.is_empty() {
        Vec::new()
    } else {
        log_parser::parse_lines(&new_content, line_offset)
    };

    let new_offset = file_len;

    Ok((
        LogTailResponse {
            log_path: path.display().to_string(),
            new_bytes,
            entries,
            offset: new_offset,
        },
        new_offset,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp_log(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn test_read_log_basic() {
        let log = write_temp_log("line1\nline2\nline3\n");
        let path = log.path().to_str().unwrap();
        let resp = read_log(100, None, None, Some(path)).unwrap();
        assert_eq!(resp.total_lines, 3);
        assert_eq!(resp.returned, 3);
    }

    #[test]
    fn test_read_log_limit() {
        let log = write_temp_log("line1\nline2\nline3\nline4\nline5\n");
        let path = log.path().to_str().unwrap();
        let resp = read_log(2, None, None, Some(path)).unwrap();
        assert_eq!(resp.returned, 2);
        assert_eq!(resp.entries[0].message, "line4");
    }

    #[test]
    fn test_read_log_filter_level() {
        let log = write_temp_log(
            "Info line\nAssets/Foo.cs(1,1): error CS0001: bad\nWarning message\n",
        );
        let path = log.path().to_str().unwrap();
        let resp = read_log(100, Some("error"), None, Some(path)).unwrap();
        assert_eq!(resp.returned, 1);
    }

    #[test]
    fn test_read_log_filter_pattern() {
        let log = write_temp_log("alpha\nbeta\ngamma\nalpha2\n");
        let path = log.path().to_str().unwrap();
        let resp = read_log(100, None, Some("alpha"), Some(path)).unwrap();
        assert_eq!(resp.returned, 2);
    }

    #[test]
    fn test_log_errors() {
        let log = write_temp_log(
            "Begin MonoManager ReloadAssembly\n\
             Assets/Scripts/Foo.cs(42,10): error CS1002: ; expected\n\
             NullReferenceException: Object ref not set\n\
               at MyClass.Start () in Assets/Scripts/MyClass.cs:5\n\
             Done\n",
        );
        let path = log.path().to_str().unwrap();
        let resp = log_errors(Some(path)).unwrap();
        assert_eq!(resp.compile_errors.len(), 1);
        assert_eq!(resp.exceptions.len(), 1);
        assert_eq!(resp.total_errors, 2);
    }

    #[test]
    fn test_log_tail() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "line1\nline2\n").unwrap();
        f.flush().unwrap();
        let path = f.path().to_str().unwrap().to_string();

        // First read
        let (resp, offset) = log_tail(0, Some(&path)).unwrap();
        assert_eq!(resp.entries.len(), 2);
        assert!(offset > 0);

        // Append more
        write!(f, "line3\n").unwrap();
        f.flush().unwrap();

        // Tail should only get new line
        let (resp2, _) = log_tail(offset, Some(&path)).unwrap();
        assert_eq!(resp2.entries.len(), 1);
        assert_eq!(resp2.entries[0].message, "line3");
    }

    #[test]
    fn test_log_tail_rotation() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "old content that is long\n").unwrap();
        f.flush().unwrap();
        let path = f.path().to_str().unwrap().to_string();

        let (_, offset) = log_tail(0, Some(&path)).unwrap();

        // Simulate rotation: truncate and write less content
        f.as_file().set_len(0).unwrap();
        f.seek(SeekFrom::Start(0)).unwrap();
        write!(f, "new\n").unwrap();
        f.flush().unwrap();

        // Should detect rotation and read from beginning
        let (resp, _) = log_tail(offset, Some(&path)).unwrap();
        assert_eq!(resp.entries[0].message, "new");
    }

    #[test]
    fn test_missing_log_file() {
        let result = read_log(100, None, None, Some("/nonexistent/Editor.log"));
        assert!(result.is_err());
    }
}
