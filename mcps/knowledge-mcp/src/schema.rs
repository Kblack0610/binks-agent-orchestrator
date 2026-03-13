//! SQLite schema initialization with FTS5 support

use rusqlite::Connection;

/// Initialize the knowledge database schema.
///
/// Creates sources, documents, chunks tables and FTS5 virtual table
/// with auto-sync triggers.
pub fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    conn.execute_batch(
        r#"
        -- Sources: configured ingestion targets
        CREATE TABLE IF NOT EXISTS sources (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            repo TEXT NOT NULL,
            base_path TEXT NOT NULL,
            source_type TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- Documents: one row per ingested file
        CREATE TABLE IF NOT EXISTS documents (
            id TEXT PRIMARY KEY,
            source_id TEXT NOT NULL,
            repo TEXT NOT NULL,
            file_path TEXT NOT NULL UNIQUE,
            relative_path TEXT NOT NULL,
            source_type TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT 'docs',
            priority INTEGER NOT NULL DEFAULT 0,
            title TEXT,
            content TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            file_mtime TEXT,
            commit_hash TEXT,
            sync_time TEXT NOT NULL,
            chunk_count INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (source_id) REFERENCES sources(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_documents_repo ON documents(repo);
        CREATE INDEX IF NOT EXISTS idx_documents_kind ON documents(kind);
        CREATE INDEX IF NOT EXISTS idx_documents_source_type ON documents(source_type);
        CREATE INDEX IF NOT EXISTS idx_documents_path ON documents(file_path);

        -- Chunks: section-level splits for FTS granularity
        CREATE TABLE IF NOT EXISTS chunks (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            heading TEXT,
            content TEXT NOT NULL,
            byte_offset INTEGER NOT NULL,
            byte_length INTEGER NOT NULL,
            FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_chunks_document ON chunks(document_id, chunk_index);

        -- FTS5 virtual table with content-sync from chunks
        CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
            content, heading,
            content='chunks', content_rowid='rowid',
            tokenize='porter unicode61'
        );

        -- Auto-sync triggers keep FTS in sync with chunks table
        CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
            INSERT INTO chunks_fts(rowid, content, heading)
            VALUES (new.rowid, new.content, new.heading);
        END;
        CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
            INSERT INTO chunks_fts(chunks_fts, rowid, content, heading)
            VALUES ('delete', old.rowid, old.content, old.heading);
        END;
        CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
            INSERT INTO chunks_fts(chunks_fts, rowid, content, heading)
            VALUES ('delete', old.rowid, old.content, old.heading);
            INSERT INTO chunks_fts(rowid, content, heading)
            VALUES (new.rowid, new.content, new.heading);
        END;
        "#,
    )
}
