//! doctl CLI wrapper module
//!
//! This module provides async functions for executing doctl CLI commands
//! and parsing their output.

pub mod error;
pub mod executor;

pub use error::DoctlError;
pub use executor::{execute_doctl_action, execute_doctl_json, execute_doctl_raw};
