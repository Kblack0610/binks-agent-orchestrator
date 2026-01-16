//! Repository type definitions
//!
//! Structs representing GitHub repository data as returned by gh CLI.

use serde::{Deserialize, Serialize};
use super::common::User;

/// Represents a GitHub repository
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    /// Repository name (without owner)
    pub name: String,

    /// Full repository name with owner (e.g., "owner/repo")
    pub name_with_owner: String,

    /// Repository description
    #[serde(default)]
    pub description: Option<String>,

    /// Repository URL on GitHub
    pub url: String,

    /// SSH URL for cloning
    #[serde(default)]
    pub ssh_url: Option<String>,

    /// HTTPS URL for cloning
    #[serde(default)]
    pub clone_url: Option<String>,

    /// Whether repository is private
    #[serde(default)]
    pub is_private: bool,

    /// Whether repository is a fork
    #[serde(default)]
    pub is_fork: bool,

    /// Whether repository is archived
    #[serde(default)]
    pub is_archived: bool,

    /// Default branch name
    #[serde(default)]
    pub default_branch_ref: Option<BranchRef>,

    /// Primary language
    #[serde(default)]
    pub primary_language: Option<Language>,

    /// Star count
    #[serde(default)]
    pub stargazer_count: Option<u32>,

    /// Fork count
    #[serde(default)]
    pub fork_count: Option<u32>,

    /// Repository owner
    #[serde(default)]
    pub owner: Option<User>,

    /// Creation timestamp (ISO 8601)
    #[serde(default)]
    pub created_at: Option<String>,

    /// Last update timestamp (ISO 8601)
    #[serde(default)]
    pub updated_at: Option<String>,

    /// Last push timestamp (ISO 8601)
    #[serde(default)]
    pub pushed_at: Option<String>,
}

/// Represents a branch reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchRef {
    /// Branch name
    pub name: String,
}

/// Represents a programming language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    /// Language name
    pub name: String,
}

impl Repository {
    /// Returns the JSON fields for repository list
    pub fn list_fields() -> &'static [&'static str] {
        &[
            "name",
            "nameWithOwner",
            "description",
            "url",
            "isPrivate",
            "isFork",
            "isArchived",
            "primaryLanguage",
            "stargazerCount",
            "updatedAt",
        ]
    }

    /// Returns the JSON fields for repository view
    pub fn view_fields() -> &'static [&'static str] {
        &[
            "name",
            "nameWithOwner",
            "description",
            "url",
            "sshUrl",
            "cloneUrl",
            "isPrivate",
            "isFork",
            "isArchived",
            "defaultBranchRef",
            "primaryLanguage",
            "stargazerCount",
            "forkCount",
            "owner",
            "createdAt",
            "updatedAt",
            "pushedAt",
        ]
    }
}
