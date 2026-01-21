//! Type definitions for GitHub entities
//!
//! This module contains Rust structs that represent GitHub entities
//! as returned by the gh CLI in JSON format.

pub mod check;
pub mod common;
pub mod issue;
pub mod pull_request;
pub mod repo;
pub mod workflow;

pub use issue::Issue;
pub use pull_request::PullRequest;
pub use repo::Repository;
pub use workflow::{Workflow, WorkflowRun};
