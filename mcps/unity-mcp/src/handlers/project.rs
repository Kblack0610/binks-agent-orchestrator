//! Project detection and info handler

use crate::detect;
use mcp_common::McpError;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Response for unity_project_info
#[derive(Serialize)]
pub struct ProjectInfoResponse {
    pub path: String,
    pub unity_version: Option<String>,
    pub packages: HashMap<String, serde_json::Value>,
}

/// Read Unity version from ProjectSettings/ProjectVersion.txt
fn read_unity_version(project_path: &Path) -> Option<String> {
    let version_file = project_path.join("ProjectSettings").join("ProjectVersion.txt");
    let content = std::fs::read_to_string(version_file).ok()?;

    // Format: m_EditorVersion: 2022.3.10f1
    for line in content.lines() {
        if let Some(version) = line.strip_prefix("m_EditorVersion:") {
            return Some(version.trim().to_string());
        }
    }
    None
}

/// Read package dependencies from Packages/manifest.json
fn read_packages(project_path: &Path) -> HashMap<String, serde_json::Value> {
    let manifest = project_path.join("Packages").join("manifest.json");
    let content = match std::fs::read_to_string(manifest) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };

    parsed
        .get("dependencies")
        .and_then(|d| d.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// Get project information.
pub fn project_info(project_path: Option<&str>) -> Result<ProjectInfoResponse, McpError> {
    let path = if let Some(p) = project_path {
        let pb = PathBuf::from(p);
        if !pb.join("Assets").is_dir() || !pb.join("ProjectSettings").is_dir() {
            return Err(McpError::invalid_params(
                format!("Not a Unity project directory: {}", p),
                None,
            ));
        }
        pb
    } else {
        detect::find_project_path(None).ok_or_else(|| {
            McpError::internal_error(
                "Could not find Unity project. Set UNITY_PROJECT_PATH env var or pass project_path parameter."
                    .to_string(),
                None,
            )
        })?
    };

    let unity_version = read_unity_version(&path);
    let packages = read_packages(&path);

    Ok(ProjectInfoResponse {
        path: path.display().to_string(),
        unity_version,
        packages,
    })
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
        fs::create_dir_all(tmp.path().join("Packages")).unwrap();

        fs::write(
            tmp.path().join("ProjectSettings").join("ProjectVersion.txt"),
            "m_EditorVersion: 2022.3.10f1\nm_EditorVersionWithRevision: 2022.3.10f1 (abc123)\n",
        )
        .unwrap();

        fs::write(
            tmp.path().join("Packages").join("manifest.json"),
            r#"{
  "dependencies": {
    "com.unity.textmeshpro": "3.0.6",
    "com.unity.ugui": "1.0.0"
  }
}"#,
        )
        .unwrap();

        tmp
    }

    #[test]
    fn test_project_info() {
        let tmp = make_unity_project();
        let resp = project_info(Some(tmp.path().to_str().unwrap())).unwrap();
        assert_eq!(resp.unity_version, Some("2022.3.10f1".to_string()));
        assert_eq!(resp.packages.len(), 2);
        assert!(resp.packages.contains_key("com.unity.textmeshpro"));
    }

    #[test]
    fn test_project_info_no_version() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("Assets")).unwrap();
        fs::create_dir_all(tmp.path().join("ProjectSettings")).unwrap();

        let resp = project_info(Some(tmp.path().to_str().unwrap())).unwrap();
        assert_eq!(resp.unity_version, None);
        assert!(resp.packages.is_empty());
    }

    #[test]
    fn test_project_info_not_unity() {
        let tmp = TempDir::new().unwrap();
        let result = project_info(Some(tmp.path().to_str().unwrap()));
        assert!(result.is_err());
    }
}
