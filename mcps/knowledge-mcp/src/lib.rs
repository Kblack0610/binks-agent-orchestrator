//! Knowledge MCP Library
//!
//! Cross-repo knowledge index with FTS5 search.
//!
//! Indexes documentation from multiple repositories into a SQLite FTS5 database
//! for unified full-text search with BM25 ranking and priority boosts.

pub mod config;
pub mod docs_store;
pub mod handlers;
pub mod ingest;
pub mod params;
pub mod project_notes;
pub mod schema;
pub mod server;
pub mod types;

pub use params::*;
pub use server::KnowledgeMcpServer;
