use rusqlite::{ffi::sqlite3_auto_extension, params, Connection, OptionalExtension, Result};
use sqlite_vec::sqlite3_vec_init;
use std::path::Path;
use std::sync::Once;

pub const EMBEDDING_DIM: usize = 384;
const SCHEMA_VERSION: &str = "1";

static REGISTER_SQLITE_VEC: Once = Once::new();

pub fn open(path: &Path) -> Result<Connection> {
    register_sqlite_vec();

    let conn = Connection::open(path)?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection> {
    register_sqlite_vec();

    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

fn register_sqlite_vec() {
    REGISTER_SQLITE_VEC.call_once(|| unsafe {
        // sqlite-vec ships as a statically linked SQLite extension in the Rust crate.
        // The documented Rust path is to register sqlite3_vec_init with SQLite's
        // auto-extension hook before opening rusqlite connections, avoiding runtime
        // load_extension paths that are brittle in signed macOS app bundles.
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    });
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS notes(
            path TEXT PRIMARY KEY,
            file_id TEXT NOT NULL,
            mtime INTEGER NOT NULL,
            content_hash TEXT NOT NULL,
            title TEXT
        );

        CREATE TABLE IF NOT EXISTS chunks(
            id INTEGER PRIMARY KEY,
            note_path TEXT NOT NULL REFERENCES notes(path) ON DELETE CASCADE,
            parent_id INTEGER NULL REFERENCES chunks(id) ON DELETE CASCADE,
            level INTEGER NOT NULL,
            heading_path TEXT NOT NULL,
            ordinal INTEGER NOT NULL,
            chunk_stable_id TEXT NOT NULL UNIQUE,
            text TEXT NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS vec_chunks USING vec0(
            chunk_id INTEGER PRIMARY KEY,
            embedding FLOAT[384]
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS fts_chunks USING fts5(
            text,
            content='chunks',
            content_rowid='id'
        );

        CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
            INSERT INTO fts_chunks(rowid, text) VALUES (new.id, new.text);
        END;

        CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
            INSERT INTO fts_chunks(fts_chunks, rowid, text)
            VALUES ('delete', old.id, old.text);
        END;

        CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
            INSERT INTO fts_chunks(fts_chunks, rowid, text)
            VALUES ('delete', old.id, old.text);
            INSERT INTO fts_chunks(rowid, text) VALUES (new.id, new.text);
        END;

        CREATE TABLE IF NOT EXISTS cache(
            key TEXT PRIMARY KEY,
            response TEXT NOT NULL,
            model_ver TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS cache_deps(
            cache_key TEXT NOT NULL REFERENCES cache(key) ON DELETE CASCADE,
            note_path TEXT NOT NULL REFERENCES notes(path) ON DELETE CASCADE,
            chunk_stable_id TEXT NOT NULL REFERENCES chunks(chunk_stable_id) ON DELETE CASCADE,
            content_hash TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS meta(
            k TEXT PRIMARY KEY,
            v TEXT NOT NULL
        );

        INSERT OR REPLACE INTO meta(k, v) VALUES ('schema_version', '1');
        INSERT OR IGNORE INTO meta(k, v) VALUES ('embedding_model', '');
        INSERT OR IGNORE INTO meta(k, v) VALUES ('embedding_dim', '384');
        INSERT OR IGNORE INTO meta(k, v) VALUES ('chunker_ver', '');
        INSERT OR IGNORE INTO meta(k, v) VALUES ('vault_id', '');
        "#,
    )?;

    Ok(())
}

pub fn upsert_note(
    conn: &Connection,
    path: &str,
    file_id: &str,
    mtime: i64,
    content_hash: &str,
    title: Option<&str>,
) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO notes(path, file_id, mtime, content_hash, title)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(path) DO UPDATE SET
            file_id = excluded.file_id,
            mtime = excluded.mtime,
            content_hash = excluded.content_hash,
            title = excluded.title
        "#,
        params![path, file_id, mtime, content_hash, title],
    )?;
    Ok(())
}

pub fn insert_chunk(
    conn: &Connection,
    note_path: &str,
    parent_id: Option<i64>,
    level: i64,
    heading_path: &str,
    ordinal: i64,
    chunk_stable_id: &str,
    text: &str,
) -> Result<i64> {
    conn.execute(
        r#"
        INSERT INTO chunks(
            note_path,
            parent_id,
            level,
            heading_path,
            ordinal,
            chunk_stable_id,
            text
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            note_path,
            parent_id,
            level,
            heading_path,
            ordinal,
            chunk_stable_id,
            text
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn insert_embedding(conn: &Connection, chunk_id: i64, embedding: &[f32]) -> Result<()> {
    validate_embedding(embedding)?;
    let embedding = f32_blob(embedding);

    conn.execute("DELETE FROM vec_chunks WHERE chunk_id = ?1", params![chunk_id])?;
    conn.execute(
        "INSERT INTO vec_chunks(chunk_id, embedding) VALUES (?1, ?2)",
        params![chunk_id, embedding],
    )?;
    Ok(())
}

pub fn vec_search(conn: &Connection, query_vec: &[f32], k: usize) -> Result<Vec<(i64, f64)>> {
    validate_embedding(query_vec)?;
    let query_vec = f32_blob(query_vec);

    let mut statement = conn.prepare(
        r#"
        SELECT chunk_id, distance
        FROM vec_chunks
        WHERE embedding MATCH ?1 AND k = ?2
        ORDER BY distance
        "#,
    )?;

    statement
        .query_map(params![query_vec, k as i64], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })?
        .collect()
}

pub fn fts_search(conn: &Connection, query: &str, k: usize) -> Result<Vec<(i64, f64)>> {
    let mut statement = conn.prepare(
        r#"
        SELECT rowid, rank
        FROM fts_chunks
        WHERE fts_chunks MATCH ?1
        ORDER BY rank
        LIMIT ?2
        "#,
    )?;

    statement
        .query_map(params![query, k as i64], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })?
        .collect()
}

pub fn meta_value(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.query_row("SELECT v FROM meta WHERE k = ?1", params![key], |row| {
        row.get(0)
    })
    .optional()
}

fn validate_embedding(embedding: &[f32]) -> Result<()> {
    if embedding.len() == EMBEDDING_DIM {
        Ok(())
    } else {
        Err(rusqlite::Error::InvalidParameterName(format!(
            "embedding must have {EMBEDDING_DIM} dimensions, got {}",
            embedding.len()
        )))
    }
}

fn f32_blob(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<f32>());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_blob_uses_four_bytes_per_float() {
        assert_eq!(f32_blob(&[1.0, 2.0]).len(), 8);
    }

    #[test]
    fn schema_version_constant_matches_migration() {
        assert_eq!(SCHEMA_VERSION, "1");
    }
}
