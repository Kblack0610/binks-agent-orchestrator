//! C/FFI bindings for the Binks Agent
//!
//! Provides C-compatible functions for embedding the agent in other languages.
//!
//! # Example (C)
//!
//! ```c
//! #include "binks.h"
//!
//! int main() {
//!     BinksAgent* agent = binks_agent_new();
//!     if (agent == NULL) {
//!         return 1;
//!     }
//!
//!     char* response = binks_agent_chat(agent, "Hello!");
//!     printf("Response: %s\n", response);
//!
//!     binks_string_free(response);
//!     binks_agent_free(agent);
//!     return 0;
//! }
//! ```

mod agent;

pub use agent::*;
