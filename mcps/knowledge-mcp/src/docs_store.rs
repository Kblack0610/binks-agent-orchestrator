//! Document store: FTS5 search, document/chunk CRUD
//!
//! All database operations go through DocStore.

use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::schema;
use crate::types::*;

/// Document store backed by SQLite with FTS5
#[derive(Clone)]
pub struct DocStore {
    conn: Arc<Mutex<Connection>>,
}

impl DocStore {
    /// Create a new DocStore, initializing the database schema
    pub fn new(db_path: PathBuf) -> Result<Self, KnowledgeError> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;

        // Initialize schema synchronously
        tokio::task::block_in_place(|| {
            let store = DocStore {
                conn: Arc::new(Mutex::new(conn)),
            };
            {
                let conn = store.conn.blocking_lock();
                schema::init_schema(&conn)?;
            }
            Ok(store)
        })
    }

    // ========================================================================
    // Source Operations
    // ========================================================================

    /// Upsert a source from config
    pub async fn upsert_source(
        &self,
        name: &str,
        repo: &str,
        base_path: &str,
        source_type: &str,
        enabled: bool,
    ) -> Result<String, KnowledgeError> {
        let conn = self.conn.lock().await;
        let now = Utc::now().to_rfc3339();

        // Check if exists
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM sources WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing_id {
            conn.execute(
                "UPDATE sources SET repo = ?1, base_path = ?2, source_type = ?3, enabled = ?4, updated_at = ?5 WHERE id = ?6",
                params![repo, base_path, source_type, enabled, now, id],
            )?;
            Ok(id)
        } else {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO sources (id, name, repo, base_path, source_type, enabled, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![id, name, repo, base_path, source_type, enabled, now, now],
            )?;
            Ok(id)
        }
    }

    /// Get source ID by name
    pub async fn get_source_id(&self, name: &str) -> Result<Option<String>, KnowledgeError> {
        let conn = self.conn.lock().await;
        Ok(conn
            .query_row(
                "SELECT id FROM sources WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()?)
    }

    // ========================================================================
    // Document Operations
    // ========================================================================

    /// Get a document's content hash by file_path (for change detection)
    pub async fn get_doc_hash(&self, file_path: &str) -> Result<Option<String>, KnowledgeError> {
        let conn = self.conn.lock().await;
        Ok(conn
            .query_row(
                "SELECT content_hash FROM documents WHERE file_path = ?1",
                params![file_path],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Get a document ID by file_path
    pub async fn get_doc_id_by_path(
        &self,
        file_path: &str,
    ) -> Result<Option<String>, KnowledgeError> {
        let conn = self.conn.lock().await;
        Ok(conn
            .query_row(
                "SELECT id FROM documents WHERE file_path = ?1",
                params![file_path],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Insert or update a document and its chunks.
    /// Deletes old chunks first (triggers handle FTS cleanup).
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_document(
        &self,
        source_id: &str,
        repo: &str,
        file_path: &str,
        relative_path: &str,
        source_type: &str,
        kind: &str,
        priority: i32,
        title: Option<&str>,
        content: &str,
        content_hash: &str,
        file_mtime: Option<&str>,
        commit_hash: Option<&str>,
        chunks: &[(Option<&str>, &str, i64, i64)], // (heading, content, byte_offset, byte_length)
    ) -> Result<bool, KnowledgeError> {
        let conn = self.conn.lock().await;
        let now = Utc::now().to_rfc3339();

        // Check if document exists
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM documents WHERE file_path = ?1",
                params![file_path],
                |row| row.get(0),
            )
            .optional()?;

        let was_update = existing_id.is_some();
        let doc_id = if let Some(id) = existing_id {
            // Delete old chunks (triggers will clean FTS)
            conn.execute("DELETE FROM chunks WHERE document_id = ?1", params![id])?;

            // Update document
            conn.execute(
                "UPDATE documents SET source_id=?1, repo=?2, relative_path=?3, source_type=?4, kind=?5, priority=?6, title=?7, content=?8, content_hash=?9, file_mtime=?10, commit_hash=?11, sync_time=?12, chunk_count=?13 WHERE id=?14",
                params![source_id, repo, relative_path, source_type, kind, priority, title, content, content_hash, file_mtime, commit_hash, now, chunks.len() as i32, id],
            )?;
            id
        } else {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO documents (id, source_id, repo, file_path, relative_path, source_type, kind, priority, title, content, content_hash, file_mtime, commit_hash, sync_time, chunk_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![id, source_id, repo, file_path, relative_path, source_type, kind, priority, title, content, content_hash, file_mtime, commit_hash, now, chunks.len() as i32],
            )?;
            id
        };

        // Insert new chunks
        for (i, (heading, chunk_content, byte_offset, byte_length)) in chunks.iter().enumerate() {
            let chunk_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO chunks (id, document_id, chunk_index, heading, content, byte_offset, byte_length) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![chunk_id, doc_id, i as i32, heading, chunk_content, byte_offset, byte_length],
            )?;
        }

        Ok(was_update)
    }

    /// Remove documents for a source that are no longer in the file set
    pub async fn remove_stale_docs(
        &self,
        source_id: &str,
        current_paths: &[String],
    ) -> Result<usize, KnowledgeError> {
        let conn = self.conn.lock().await;

        // Get all doc file_paths for this source
        let mut stmt = conn.prepare("SELECT id, file_path FROM documents WHERE source_id = ?1")?;
        let existing: Vec<(String, String)> = stmt
            .query_map(params![source_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut removed = 0;
        for (id, path) in &existing {
            if !current_paths.contains(path) {
                // Delete chunks first (triggers clean FTS), then document
                conn.execute("DELETE FROM chunks WHERE document_id = ?1", params![id])?;
                conn.execute("DELETE FROM documents WHERE id = ?1", params![id])?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    // ========================================================================
    // Search Operations
    // ========================================================================

    /// FTS5 search over document chunks with BM25 ranking and priority boost
    pub async fn search(
        &self,
        query: &str,
        repo: Option<&str>,
        kind: Option<&str>,
        limit: u32,
    ) -> Result<(Vec<SearchResult>, usize), KnowledgeError> {
        let conn = self.conn.lock().await;
        let limit = limit.min(50);

        // Build the WHERE clause for filters
        let mut conditions = Vec::new();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        // FTS query is parameter ?1
        bind_values.push(Box::new(query.to_string()));

        let mut param_idx = 2;

        if let Some(r) = repo {
            conditions.push(format!("d.repo = ?{param_idx}"));
            bind_values.push(Box::new(r.to_string()));
            param_idx += 1;
        }
        if let Some(k) = kind {
            conditions.push(format!("d.kind = ?{param_idx}"));
            bind_values.push(Box::new(k.to_string()));
            param_idx += 1;
        }

        // Limit
        bind_values.push(Box::new(limit));

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("AND {}", conditions.join(" AND "))
        };

        let sql = format!(
            r#"
            SELECT
                c.id AS chunk_id,
                c.document_id,
                d.repo,
                d.relative_path,
                c.heading,
                snippet(chunks_fts, 0, '>>>', '<<<', '...', 64) AS snippet,
                d.kind,
                d.priority,
                bm25(chunks_fts) AS bm25_score,
                d.priority,
                d.sync_time,
                d.commit_hash,
                d.file_mtime,
                s.name AS source_name
            FROM chunks_fts
            JOIN chunks c ON c.rowid = chunks_fts.rowid
            JOIN documents d ON c.document_id = d.id
            JOIN sources s ON d.source_id = s.id
            WHERE chunks_fts MATCH ?1
            {where_clause}
            ORDER BY bm25(chunks_fts) ASC, d.priority DESC
            LIMIT ?{param_idx}
            "#
        );

        let mut stmt = conn.prepare(&sql)?;

        let refs: Vec<&dyn rusqlite::types::ToSql> =
            bind_values.iter().map(|b| b.as_ref()).collect();

        let results: Vec<SearchResult> = stmt
            .query_map(refs.as_slice(), |row| {
                let bm25_score: f64 = row.get(8)?;
                let sync_time: String = row.get(10)?;
                let file_mtime: Option<String> = row.get(12)?;

                // Check staleness: file_mtime > sync_time
                let stale = file_mtime
                    .as_ref()
                    .map(|mt| mt.as_str() > sync_time.as_str())
                    .unwrap_or(false);

                Ok(SearchResult {
                    chunk_id: row.get(0)?,
                    doc_id: row.get(1)?,
                    repo: row.get(2)?,
                    relative_path: row.get(3)?,
                    heading: row.get(4)?,
                    snippet: row.get(5)?,
                    kind: row.get(6)?,
                    priority: row.get(7)?,
                    rank: -bm25_score, // negate for display (higher = better)
                    sync_time,
                    commit_hash: row.get(11)?,
                    source_name: row.get(13)?,
                    stale,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let total = results.len();
        Ok((results, total))
    }

    // ========================================================================
    // Get Document
    // ========================================================================

    /// Retrieve a document by ID or by repo+path
    pub async fn get_document(
        &self,
        doc_id: Option<&str>,
        repo: Option<&str>,
        path: Option<&str>,
        chunk_range: Option<[u32; 2]>,
    ) -> Result<GetDocResponse, KnowledgeError> {
        let conn = self.conn.lock().await;

        let select = "SELECT d.id, d.repo, d.relative_path, d.kind, d.priority, d.title, d.content_hash, d.sync_time, d.commit_hash, d.chunk_count, d.file_mtime, s.name AS source_name FROM documents d JOIN sources s ON d.source_id = s.id";

        // Find the document
        let document: DocumentInfo = if let Some(id) = doc_id {
            conn.query_row(
                &format!("{select} WHERE d.id = ?1"),
                params![id],
                doc_info_from_row,
            )
            .map_err(|_| KnowledgeError::NotFound("Document not found by ID".into()))?
        } else if let (Some(r), Some(p)) = (repo, path) {
            conn.query_row(
                &format!("{select} WHERE d.repo = ?1 AND d.relative_path = ?2"),
                params![r, p],
                doc_info_from_row,
            )
            .map_err(|_| KnowledgeError::NotFound(format!("Document not found: {r}/{p}")))?
        } else {
            return Err(KnowledgeError::Other(
                "Must provide doc_id or both repo and path".into(),
            ));
        };

        // Fetch chunks
        let (offset, limit, has_range) = if let Some([start, end]) = chunk_range {
            (start, (end - start + 1), true)
        } else {
            (0u32, 20u32, false)
        };

        let mut stmt = conn.prepare(
            "SELECT id, chunk_index, heading, content FROM chunks WHERE document_id = ?1 ORDER BY chunk_index LIMIT ?2 OFFSET ?3",
        )?;

        let chunks: Vec<ChunkInfo> = stmt
            .query_map(params![document.id, limit, offset], |row| {
                Ok(ChunkInfo {
                    id: row.get(0)?,
                    chunk_index: row.get(1)?,
                    heading: row.get(2)?,
                    content: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let truncated = !has_range && document.chunk_count > 20 && chunks.len() as i32 >= 20;

        Ok(GetDocResponse {
            document,
            chunks,
            truncated,
        })
    }

    // ========================================================================
    // Status Operations
    // ========================================================================

    /// Get sync status for all sources
    pub async fn get_sync_status(&self) -> Result<Vec<SourceStatus>, KnowledgeError> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            r#"
            SELECT
                s.name,
                s.repo,
                s.enabled,
                COUNT(DISTINCT d.id) as doc_count,
                COUNT(c.id) as chunk_count,
                MAX(d.sync_time) as newest_sync,
                MIN(d.sync_time) as oldest_sync
            FROM sources s
            LEFT JOIN documents d ON d.source_id = s.id
            LEFT JOIN chunks c ON c.document_id = d.id
            GROUP BY s.id
            ORDER BY s.name
            "#,
        )?;

        let mut statuses: Vec<SourceStatus> = stmt
            .query_map([], |row| {
                Ok(SourceStatus {
                    name: row.get(0)?,
                    repo: row.get(1)?,
                    enabled: row.get::<_, i32>(2)? != 0,
                    document_count: row.get::<_, i64>(3)? as usize,
                    chunk_count: row.get::<_, i64>(4)? as usize,
                    newest_sync: row.get(5)?,
                    oldest_sync: row.get(6)?,
                    last_sync_time: None,
                    stale_count: 0,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        // Fill in last_sync_time and stale_count
        for status in &mut statuses {
            status.last_sync_time = status.newest_sync.clone();

            // Count stale docs
            let stale: i64 = conn.query_row(
                "SELECT COUNT(*) FROM documents d JOIN sources s ON d.source_id = s.id WHERE s.name = ?1 AND d.file_mtime IS NOT NULL AND d.file_mtime > d.sync_time",
                params![status.name],
                |row| row.get(0),
            )?;
            status.stale_count = stale as usize;
        }

        Ok(statuses)
    }

    /// Get doc count for a source by name
    pub async fn get_source_doc_count(&self, source_name: &str) -> Result<usize, KnowledgeError> {
        let conn = self.conn.lock().await;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM documents d JOIN sources s ON d.source_id = s.id WHERE s.name = ?1",
                params![source_name],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count as usize)
    }
}

/// Map a row to DocumentInfo (for get_document queries)
/// Expected columns: id, repo, relative_path, kind, priority, title,
///   content_hash, sync_time, commit_hash, chunk_count, file_mtime, source_name
fn doc_info_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DocumentInfo> {
    let sync_time: String = row.get(7)?;
    let file_mtime: Option<String> = row.get(10)?;
    let stale = file_mtime
        .as_ref()
        .map(|mt| mt.as_str() > sync_time.as_str())
        .unwrap_or(false);

    Ok(DocumentInfo {
        id: row.get(0)?,
        repo: row.get(1)?,
        relative_path: row.get(2)?,
        kind: row.get(3)?,
        priority: row.get(4)?,
        title: row.get(5)?,
        content_hash: row.get(6)?,
        sync_time,
        commit_hash: row.get(8)?,
        chunk_count: row.get(9)?,
        source_name: row.get(11)?,
        stale,
    })
}

/// Extension trait for optional query results
trait SqliteOptional<T> {
    fn optional(self) -> rusqlite::Result<Option<T>>;
}

impl<T> SqliteOptional<T> for rusqlite::Result<T> {
    fn optional(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_store() -> DocStore {
        let dir = tempdir().unwrap();
        let db_path = dir.keep().join("test_knowledge.db");
        DocStore::new(db_path).unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fts5_smoke() {
        let store = test_store();

        // Upsert a source
        let source_id = store
            .upsert_source("test-repo", "test-repo", "/tmp/test", "docs", true)
            .await
            .unwrap();

        // Insert a document with chunks
        let chunks: &[(Option<&str>, &str, i64, i64)] = &[
            (
                Some("Architecture Overview"),
                "This document describes the system architecture and deployment strategy.",
                0i64,
                70i64,
            ),
            (
                Some("Database Design"),
                "We use PostgreSQL for the primary data store with Redis for caching.",
                71i64,
                68i64,
            ),
        ];

        store
            .upsert_document(
                &source_id,
                "test-repo",
                "/tmp/test/docs/ARCHITECTURE.md",
                "docs/ARCHITECTURE.md",
                "docs",
                "architecture",
                7,
                Some("Architecture Overview"),
                "full content here",
                "abc123hash",
                None,
                Some("deadbeef"),
                chunks,
            )
            .await
            .unwrap();

        // Search for architecture
        let (results, total) = store.search("architecture", None, None, 10).await.unwrap();
        assert!(total > 0, "Expected search results for 'architecture'");
        assert_eq!(results[0].repo, "test-repo");
        assert_eq!(results[0].kind, "architecture");
        assert!(results[0].commit_hash.as_deref() == Some("deadbeef"));

        // Search for PostgreSQL
        let (results, _) = store.search("PostgreSQL", None, None, 10).await.unwrap();
        assert!(!results.is_empty(), "Expected results for 'PostgreSQL'");
        assert_eq!(results[0].heading.as_deref(), Some("Database Design"));

        // Search with repo filter
        let (results, _) = store
            .search("architecture", Some("nonexistent"), None, 10)
            .await
            .unwrap();
        assert!(results.is_empty());

        // Search with kind filter
        let (results, _) = store
            .search("architecture", None, Some("architecture"), 10)
            .await
            .unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_document_by_repo_path() {
        let store = test_store();

        let source_id = store
            .upsert_source("myrepo", "myrepo", "/tmp/myrepo", "docs", true)
            .await
            .unwrap();

        let chunks = vec![(None::<&str>, "Some content", 0i64, 12i64)];

        store
            .upsert_document(
                &source_id,
                "myrepo",
                "/tmp/myrepo/README.md",
                "README.md",
                "docs",
                "docs",
                10,
                Some("My Readme"),
                "Some content",
                "hash123",
                None,
                None,
                &chunks,
            )
            .await
            .unwrap();

        // Retrieve by repo+path
        let response = store
            .get_document(None, Some("myrepo"), Some("README.md"), None)
            .await
            .unwrap();

        assert_eq!(response.document.repo, "myrepo");
        assert_eq!(response.document.relative_path, "README.md");
        assert_eq!(response.document.priority, 10);
        assert_eq!(response.chunks.len(), 1);
        assert!(!response.truncated);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_hash_based_skip() {
        let store = test_store();

        let source_id = store
            .upsert_source("repo", "repo", "/tmp/repo", "docs", true)
            .await
            .unwrap();

        // First insert
        let chunks = vec![(None::<&str>, "content", 0i64, 7i64)];
        let was_update = store
            .upsert_document(
                &source_id,
                "repo",
                "/tmp/repo/f.md",
                "f.md",
                "docs",
                "docs",
                0,
                None,
                "content",
                "hash1",
                None,
                None,
                &chunks,
            )
            .await
            .unwrap();
        assert!(!was_update);

        // Check hash
        let hash = store.get_doc_hash("/tmp/repo/f.md").await.unwrap();
        assert_eq!(hash, Some("hash1".to_string()));

        // Second insert (update)
        let was_update = store
            .upsert_document(
                &source_id,
                "repo",
                "/tmp/repo/f.md",
                "f.md",
                "docs",
                "docs",
                0,
                None,
                "new content",
                "hash2",
                None,
                None,
                &chunks,
            )
            .await
            .unwrap();
        assert!(was_update);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_sync_status() {
        let store = test_store();

        store
            .upsert_source("src1", "repo1", "/tmp/r1", "docs", true)
            .await
            .unwrap();

        let statuses = store.get_sync_status().await.unwrap();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].name, "src1");
        assert_eq!(statuses[0].document_count, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_chunk_capping() {
        let store = test_store();

        let source_id = store
            .upsert_source("repo", "repo", "/tmp/repo", "docs", true)
            .await
            .unwrap();

        // Create 25 chunks
        let chunks: Vec<(Option<&str>, &str, i64, i64)> = (0..25)
            .map(|i| (None, "chunk content", (i * 100) as i64, 13i64))
            .collect();

        store
            .upsert_document(
                &source_id,
                "repo",
                "/tmp/repo/big.md",
                "big.md",
                "docs",
                "docs",
                0,
                None,
                "big content",
                "bighash",
                None,
                None,
                &chunks,
            )
            .await
            .unwrap();

        // Get doc without range - should be capped at 20
        let response = store
            .get_document(None, Some("repo"), Some("big.md"), None)
            .await
            .unwrap();
        assert_eq!(response.chunks.len(), 20);
        assert!(response.truncated);

        // Get doc with explicit range
        let response = store
            .get_document(None, Some("repo"), Some("big.md"), Some([0, 24]))
            .await
            .unwrap();
        assert_eq!(response.chunks.len(), 25);
        assert!(!response.truncated);
    }
}
