//! Common types shared across GitHub entities
//!
//! This module contains types that are used by multiple GitHub entities,
//! such as users, labels, and milestones.

use serde::{Deserialize, Serialize};

/// Represents a GitHub user (author, assignee, reviewer, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// GitHub login/username
    pub login: String,

    /// User's display name (may be empty)
    #[serde(default)]
    pub name: Option<String>,
}

/// Represents a GitHub label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    /// Label name
    pub name: String,

    /// Label color (hex without #)
    #[serde(default)]
    pub color: Option<String>,

    /// Label description
    #[serde(default)]
    pub description: Option<String>,
}

/// Represents a GitHub milestone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// Milestone title
    pub title: String,

    /// Milestone number
    pub number: u32,

    /// Milestone state (open/closed)
    #[serde(default)]
    pub state: Option<String>,
}

/// Represents a repository reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRef {
    /// Repository name with owner (e.g., "owner/repo")
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
}
