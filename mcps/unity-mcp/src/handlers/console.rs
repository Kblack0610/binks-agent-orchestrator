//! Console log handler for Unity runtime Debug.Log/LogWarning/LogError entries

use crate::handlers::logs::resolve_log_path;
use crate::runtime_parser::{self, RuntimeLogEntry};
use mcp_common::McpError;
use regex::Regex;
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};

/// Response for unity_console_log
#[derive(Serialize)]
pub struct ConsoleLogResponse {
    pub log_path: String,
    pub total_entries: usize,
    pub returned: usize,
    pub entries: Vec<RuntimeLogEntry>,
}

/// Initial chunk size to read from end of file (512KB)
const INITIAL_CHUNK_SIZE: u64 = 512 * 1024;

/// Maximum chunk size (8MB) to prevent reading enormous files entirely
const MAX_CHUNK_SIZE: u64 = 8 * 1024 * 1024;

/// Read Unity runtime console log entries.
///
/// Reads the log file from the end in chunks to efficiently find the requested
/// number of entries without loading the entire file.
pub fn console_log(
    count: usize,
    level: Option<&str>,
    pattern: Option<&str>,
    include_stack_traces: bool,
    log_path: Option<&str>,
) -> Result<ConsoleLogResponse, McpError> {
    let path = resolve_log_path(log_path)?;

    let pattern_re = pattern
        .map(|p| {
            Regex::new(p).map_err(|e| {
                McpError::invalid_params(format!("Invalid regex pattern: {}", e), None)
            })
        })
        .transpose()?;

    let level_filter = level.map(|l| l.to_lowercase());

    // Read file from end in expanding chunks until we have enough entries
    let mut file = std::fs::File::open(&path).map_err(|e| {
        McpError::internal_error(format!("Failed to open log file: {}", e), None)
    })?;

    let metadata = file.metadata().map_err(|e| {
        McpError::internal_error(format!("Failed to read file metadata: {}", e), None)
    })?;

    let file_len = metadata.len();
    let mut chunk_size = INITIAL_CHUNK_SIZE.min(file_len);
    let mut entries: Vec<RuntimeLogEntry>;

    loop {
        let seek_pos = file_len.saturating_sub(chunk_size);

        file.seek(SeekFrom::Start(seek_pos)).map_err(|e| {
            McpError::internal_error(format!("Failed to seek in log file: {}", e), None)
        })?;

        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| {
            McpError::internal_error(format!("Failed to read log content: {}", e), None)
        })?;

        // If we didn't start at the beginning, skip the first partial line
        let text = if seek_pos > 0 {
            match content.find('\n') {
                Some(pos) => &content[pos + 1..],
                None => &content,
            }
        } else {
            &content
        };

        // Calculate approximate start line number
        let start_line = if seek_pos > 0 {
            // Rough estimate — not exact but good enough for display
            0
        } else {
            0
        };

        entries = runtime_parser::parse_runtime_entries(text, start_line);

        // Apply filters
        if let Some(ref lf) = level_filter {
            entries.retain(|e| e.level.matches_filter(lf));
        }

        if let Some(ref re) = pattern_re {
            entries.retain(|e| re.is_match(&e.message));
        }

        // If we have enough entries or we've read the whole file, stop
        if entries.len() >= count || chunk_size >= file_len || chunk_size >= MAX_CHUNK_SIZE {
            break;
        }

        // Double the chunk size and try again
        chunk_size = (chunk_size * 2).min(file_len).min(MAX_CHUNK_SIZE);
    }

    let total_entries = entries.len();

    // Take the last `count` entries
    if entries.len() > count {
        entries = entries.split_off(entries.len() - count);
    }

    // Strip stack traces if not requested
    if !include_stack_traces {
        for entry in &mut entries {
            entry.stack_trace.clear();
        }
    }

    let returned = entries.len();

    Ok(ConsoleLogResponse {
        log_path: path.display().to_string(),
        total_entries,
        returned,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_parser::RuntimeLogLevel;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp_log(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const SAMPLE_MULTI_ENTRY_LOG: &str = "\
Initializing Unity editor...
Player connected
this is info
UnityEngine.Debug:ExtractStackTraceNoAlloc (byte*,int,string)
UnityEngine.StackTraceUtility:ExtractStackTrace ()
UnityEngine.DebugLogHandler:Internal_Log (UnityEngine.LogType,UnityEngine.LogOption,string,UnityEngine.Object)
UnityEngine.DebugLogHandler:LogFormat (UnityEngine.LogType,UnityEngine.Object,string,object[])
UnityEngine.Logger:Log (UnityEngine.LogType,object)
UnityEngine.Debug:Log (object)
GameManager:Start () (at Assets/Scripts/GameManager.cs:10)

(Filename: Assets/Scripts/GameManager.cs Line: 10)
watch out!
UnityEngine.Debug:ExtractStackTraceNoAlloc (byte*,int,string)
UnityEngine.StackTraceUtility:ExtractStackTrace ()
UnityEngine.DebugLogHandler:Internal_Log (UnityEngine.LogType,UnityEngine.LogOption,string,UnityEngine.Object)
UnityEngine.DebugLogHandler:LogFormat (UnityEngine.LogType,UnityEngine.Object,string,object[])
UnityEngine.Logger:Log (UnityEngine.LogType,object)
UnityEngine.Debug:LogWarning (object)
GameManager:Update () (at Assets/Scripts/GameManager.cs:20)

(Filename: Assets/Scripts/GameManager.cs Line: 20)
critical failure
UnityEngine.Debug:ExtractStackTraceNoAlloc (byte*,int,string)
UnityEngine.StackTraceUtility:ExtractStackTrace ()
UnityEngine.DebugLogHandler:Internal_Log (UnityEngine.LogType,UnityEngine.LogOption,string,UnityEngine.Object)
UnityEngine.DebugLogHandler:LogFormat (UnityEngine.LogType,UnityEngine.Object,string,object[])
UnityEngine.Logger:Log (UnityEngine.LogType,object)
UnityEngine.Debug:LogError (object)
PlayerController:Die () (at Assets/Scripts/PlayerController.cs:99)

(Filename: Assets/Scripts/PlayerController.cs Line: 99)
";

    #[test]
    fn test_console_log_basic() {
        let log = write_temp_log(SAMPLE_MULTI_ENTRY_LOG);
        let path = log.path().to_str().unwrap();
        let resp = console_log(50, None, None, false, Some(path)).unwrap();
        // Should find runtime entries (log, warning, error blocks) plus any non-block lines
        assert!(resp.returned >= 3);
    }

    #[test]
    fn test_console_log_filter_level() {
        let log = write_temp_log(SAMPLE_MULTI_ENTRY_LOG);
        let path = log.path().to_str().unwrap();
        let resp = console_log(50, Some("error"), None, false, Some(path)).unwrap();
        // Only the error entry
        assert!(resp.returned >= 1);
        for entry in &resp.entries {
            assert_eq!(entry.level, RuntimeLogLevel::Error);
        }
    }

    #[test]
    fn test_console_log_filter_pattern() {
        let log = write_temp_log(SAMPLE_MULTI_ENTRY_LOG);
        let path = log.path().to_str().unwrap();
        let resp = console_log(50, None, Some("critical"), false, Some(path)).unwrap();
        assert_eq!(resp.returned, 1);
        assert!(resp.entries[0].message.contains("critical"));
    }

    #[test]
    fn test_console_log_with_stack_traces() {
        let log = write_temp_log(SAMPLE_MULTI_ENTRY_LOG);
        let path = log.path().to_str().unwrap();

        // Without stack traces
        let resp = console_log(50, None, None, false, Some(path)).unwrap();
        for entry in &resp.entries {
            assert!(entry.stack_trace.is_empty());
        }

        // With stack traces
        let resp = console_log(50, None, None, true, Some(path)).unwrap();
        let runtime_entries: Vec<_> = resp
            .entries
            .iter()
            .filter(|e| e.source_file.is_some())
            .collect();
        for entry in &runtime_entries {
            assert!(!entry.stack_trace.is_empty());
        }
    }

    #[test]
    fn test_console_log_count_limit() {
        let log = write_temp_log(SAMPLE_MULTI_ENTRY_LOG);
        let path = log.path().to_str().unwrap();
        let resp = console_log(1, None, None, false, Some(path)).unwrap();
        assert_eq!(resp.returned, 1);
    }

    #[test]
    fn test_console_log_missing_file() {
        let result = console_log(50, None, None, false, Some("/nonexistent/Editor.log"));
        assert!(result.is_err());
    }
}
