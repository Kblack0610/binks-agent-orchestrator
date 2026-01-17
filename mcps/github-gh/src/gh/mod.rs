//! gh CLI wrapper module
//!
//! This module provides async functions for executing gh CLI commands
//! and parsing their output.

pub mod error;
pub mod executor;

pub use error::{GhError, GhResult};
pub use executor::{check_gh_available, execute_gh_action, execute_gh_json, execute_gh_raw};
