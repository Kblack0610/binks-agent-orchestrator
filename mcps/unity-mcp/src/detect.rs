//! Platform-specific detection for Unity log files and project paths

use std::path::{Path, PathBuf};

/// Find the Unity Editor.log file path.
///
/// Priority:
/// 1. `UNITY_LOG_PATH` env var
/// 2. Platform-specific default location
pub fn find_log_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("UNITY_LOG_PATH") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    platform_log_path()
}

/// Return the platform-specific default Editor.log path.
fn platform_log_path() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        dirs::config_dir().map(|d| d.join("unity3d").join("Editor.log"))
    }

    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|d| d.join("Library/Logs/Unity/Editor.log"))
    }

    #[cfg(target_os = "windows")]
    {
        dirs::data_local_dir().map(|d| d.join("Unity").join("Editor").join("Editor.log"))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

/// Find a Unity project root by walking up from a starting directory.
///
/// Priority:
/// 1. `UNITY_PROJECT_PATH` env var
/// 2. Walk up from `start_dir` looking for `Assets/` + `ProjectSettings/`
pub fn find_project_path(start_dir: Option<&Path>) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("UNITY_PROJECT_PATH") {
        let p = PathBuf::from(&path);
        if is_unity_project(&p) {
            return Some(p);
        }
    }

    let start = if let Some(dir) = start_dir {
        dir.to_path_buf()
    } else {
        std::env::current_dir().ok()?
    };

    let mut current = start.as_path();
    loop {
        if is_unity_project(current) {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

/// Check if a directory looks like a Unity project root.
fn is_unity_project(dir: &Path) -> bool {
    dir.join("Assets").is_dir() && dir.join("ProjectSettings").is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_unity_project() -> TempDir {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("Assets")).unwrap();
        fs::create_dir_all(tmp.path().join("ProjectSettings")).unwrap();
        tmp
    }

    #[test]
    fn test_is_unity_project() {
        let tmp = make_unity_project();
        assert!(is_unity_project(tmp.path()));
    }

    #[test]
    fn test_is_not_unity_project() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_unity_project(tmp.path()));
    }

    #[test]
    fn test_find_project_walks_up() {
        let tmp = make_unity_project();
        let nested = tmp.path().join("Assets").join("Scripts");
        fs::create_dir_all(&nested).unwrap();
        let found = find_project_path(Some(&nested));
        assert_eq!(found, Some(tmp.path().to_path_buf()));
    }

    #[test]
    fn test_find_project_env_var() {
        let tmp = make_unity_project();
        unsafe { std::env::set_var("UNITY_PROJECT_PATH", tmp.path()) };
        let found = find_project_path(None);
        assert_eq!(found, Some(tmp.path().to_path_buf()));
        unsafe { std::env::remove_var("UNITY_PROJECT_PATH") };
    }
}
