//! Environment context gathering for system prompts

use std::path::PathBuf;
use std::process::Command;

/// Gathers environment context for the agent
pub struct EnvironmentContext {
    pub cwd: PathBuf,
    pub git_repo: Option<String>,
    pub git_branch: Option<String>,
    pub git_clean: Option<bool>,
}

impl EnvironmentContext {
    /// Gather context from the current environment
    pub fn gather() -> Self {
        let cwd = std::env::current_dir().unwrap_or_default();
        let (git_repo, git_branch, git_clean) = get_git_info();

        Self {
            cwd,
            git_repo,
            git_branch,
            git_clean,
        }
    }

    /// Convert to a system prompt string
    pub fn to_system_prompt(&self) -> String {
        let mut prompt = String::from(
            "You are an AI agent with access to tools.\n\n\
             IMPORTANT: Check the Environment section below before using tools for \
             basic questions about your current location or repository.\n\n\
             Environment:\n",
        );

        prompt.push_str(&format!("- Working directory: {}\n", self.cwd.display()));

        if let Some(ref repo) = self.git_repo {
            let mut git_info = format!("- Repository: {}", repo);
            if let Some(ref branch) = self.git_branch {
                git_info.push_str(&format!(" (branch: {}", branch));
                if let Some(clean) = self.git_clean {
                    git_info.push_str(if clean { ", clean)" } else { ", modified)" });
                } else {
                    git_info.push(')');
                }
            }
            prompt.push_str(&git_info);
            prompt.push('\n');
        }

        prompt
    }
}

/// Get git repository info using git commands
fn get_git_info() -> (Option<String>, Option<String>, Option<bool>) {
    // Check if we're in a git repo
    let in_repo = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !in_repo {
        return (None, None, None);
    }

    // Get repo name from remote or folder
    let repo_name = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|url| extract_repo_name(&url))
        .or_else(|| {
            // Fallback to top-level folder name
            Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|p| {
                    PathBuf::from(p.trim())
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                })
        });

    // Get current branch
    let branch = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Check if clean (no modified files)
    let clean = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| o.stdout.is_empty());

    (repo_name, branch, clean)
}

/// Extract repo name from git remote URL
fn extract_repo_name(url: &str) -> String {
    let url = url.trim();
    // Handle SSH format: git@github.com:owner/repo.git
    // Handle HTTPS format: https://github.com/owner/repo.git
    url.rsplit('/')
        .next()
        .or_else(|| url.rsplit(':').next())
        .map(|s| s.trim_end_matches(".git"))
        .unwrap_or(url)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name_https() {
        assert_eq!(
            extract_repo_name("https://github.com/owner/repo.git"),
            "repo"
        );
        assert_eq!(
            extract_repo_name("https://github.com/owner/repo"),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_ssh() {
        assert_eq!(
            extract_repo_name("git@github.com:owner/repo.git"),
            "repo"
        );
        assert_eq!(
            extract_repo_name("git@github.com:owner/repo"),
            "repo"
        );
    }

    #[test]
    fn test_to_system_prompt() {
        let ctx = EnvironmentContext {
            cwd: PathBuf::from("/home/user/project"),
            git_repo: Some("my-project".to_string()),
            git_branch: Some("main".to_string()),
            git_clean: Some(true),
        };

        let prompt = ctx.to_system_prompt();
        assert!(prompt.contains("Working directory: /home/user/project"));
        assert!(prompt.contains("Repository: my-project"));
        assert!(prompt.contains("branch: main"));
        assert!(prompt.contains("clean"));
    }

    #[test]
    fn test_extract_repo_name_gitlab() {
        assert_eq!(
            extract_repo_name("https://gitlab.com/group/subgroup/repo.git"),
            "repo"
        );
        assert_eq!(
            extract_repo_name("git@gitlab.com:group/subgroup/repo.git"),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_edge_cases() {
        // Whitespace
        assert_eq!(
            extract_repo_name("  https://github.com/o/repo.git  "),
            "repo"
        );
        // No .git suffix
        assert_eq!(
            extract_repo_name("https://github.com/owner/my-repo"),
            "my-repo"
        );
        // Just a name (fallback case)
        assert_eq!(extract_repo_name("local-repo"), "local-repo");
    }

    #[test]
    fn test_to_system_prompt_no_git() {
        let ctx = EnvironmentContext {
            cwd: PathBuf::from("/tmp/random"),
            git_repo: None,
            git_branch: None,
            git_clean: None,
        };

        let prompt = ctx.to_system_prompt();
        assert!(prompt.contains("Working directory: /tmp/random"));
        assert!(!prompt.contains("Repository:"));
    }

    #[test]
    fn test_to_system_prompt_modified() {
        let ctx = EnvironmentContext {
            cwd: PathBuf::from("/home/user/project"),
            git_repo: Some("dirty-repo".to_string()),
            git_branch: Some("feature".to_string()),
            git_clean: Some(false),
        };

        let prompt = ctx.to_system_prompt();
        assert!(prompt.contains("modified)"));
        assert!(!prompt.contains("clean)"));
    }

    #[test]
    fn test_to_system_prompt_no_branch() {
        // Detached HEAD state - has repo but no branch
        let ctx = EnvironmentContext {
            cwd: PathBuf::from("/home/user/project"),
            git_repo: Some("detached-repo".to_string()),
            git_branch: None,
            git_clean: Some(true),
        };

        let prompt = ctx.to_system_prompt();
        assert!(prompt.contains("Repository: detached-repo"));
        assert!(!prompt.contains("branch:"));
    }

    #[test]
    fn test_gather_returns_valid_cwd() {
        // Integration test - should always return a valid cwd
        let ctx = EnvironmentContext::gather();
        assert!(!ctx.cwd.as_os_str().is_empty());
    }
}
