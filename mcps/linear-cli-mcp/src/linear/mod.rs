//! Linear CLI wrapper
//!
//! Async executor and error types for the `linear` CLI.

pub mod error;
pub mod executor;

pub use error::LinearError;
pub use executor::execute_linear;
#[cfg(feature = "documents")]
pub use executor::execute_linear_json;
