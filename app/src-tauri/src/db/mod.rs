use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::{c_char, c_double, c_int, c_void};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr::{self, NonNull};
use std::time::{SystemTime, UNIX_EPOCH};

pub const EMBEDDING_DIM: usize = 384;
const EMBEDDING_BYTES: usize = EMBEDDING_DIM * std::mem::size_of::<f32>();

type SqliteDestructor = Option<unsafe extern "C" fn(*mut c_void)>;

#[allow(non_camel_case_types)]
enum sqlite3 {}

#[allow(non_camel_case_types)]
enum sqlite3_stmt {}

#[link(name = "sqlite3")]
extern "C" {
    fn sqlite3_open_v2(
        filename: *const c_char,
        pp_db: *mut *mut sqlite3,
        flags: c_int,
        z_vfs: *const c_char,
    ) -> c_int;
    fn sqlite3_close(db: *mut sqlite3) -> c_int;
    fn sqlite3_exec(
        db: *mut sqlite3,
        sql: *const c_char,
        callback: Option<
            unsafe extern "C" fn(*mut c_void, c_int, *mut *mut c_char, *mut *mut c_char) -> c_int,
        >,
        arg: *mut c_void,
        errmsg: *mut *mut c_char,
    ) -> c_int;
    fn sqlite3_free(ptr: *mut c_void);
    fn sqlite3_errmsg(db: *mut sqlite3) -> *const c_char;
    fn sqlite3_prepare_v2(
        db: *mut sqlite3,
        sql: *const c_char,
        n_byte: c_int,
        pp_stmt: *mut *mut sqlite3_stmt,
        pz_tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_finalize(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_step(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_bind_null(stmt: *mut sqlite3_stmt, index: c_int) -> c_int;
    fn sqlite3_bind_int64(stmt: *mut sqlite3_stmt, index: c_int, value: i64) -> c_int;
    fn sqlite3_bind_text(
        stmt: *mut sqlite3_stmt,
        index: c_int,
        value: *const c_char,
        n: c_int,
        destructor: SqliteDestructor,
    ) -> c_int;
    fn sqlite3_bind_blob(
        stmt: *mut sqlite3_stmt,
        index: c_int,
        value: *const c_void,
        n: c_int,
        destructor: SqliteDestructor,
    ) -> c_int;
    fn sqlite3_column_int64(stmt: *mut sqlite3_stmt, index: c_int) -> i64;
    fn sqlite3_column_double(stmt: *mut sqlite3_stmt, index: c_int) -> c_double;
    fn sqlite3_column_text(stmt: *mut sqlite3_stmt, index: c_int) -> *const u8;
    fn sqlite3_column_blob(stmt: *mut sqlite3_stmt, index: c_int) -> *const c_void;
    fn sqlite3_column_bytes(stmt: *mut sqlite3_stmt, index: c_int) -> c_int;
    fn sqlite3_last_insert_rowid(db: *mut sqlite3) -> i64;
    fn sqlite3_changes(db: *mut sqlite3) -> c_int;
}

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_OPEN_READWRITE: c_int = 0x0000_0002;
const SQLITE_OPEN_CREATE: c_int = 0x0000_0004;
const SQLITE_OPEN_FULLMUTEX: c_int = 0x0001_0000;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    message: String,
}

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub struct Connection {
    raw: NonNull<sqlite3>,
}

// Connections are opened with SQLITE_OPEN_FULLMUTEX and app access is guarded by
// a single-writer Mutex in the indexer state.
unsafe impl Send for Connection {}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            sqlite3_close(self.raw.as_ptr());
        }
    }
}

pub fn open(path: &Path) -> Result<Connection> {
    let path = cstring_from_path(path)?;
    let conn = open_raw(&path)?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection> {
    let path = CString::new(":memory:").expect("static memory path has no nul");
    let conn = open_raw(&path)?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

fn open_raw(path: &CString) -> Result<Connection> {
    let mut db = ptr::null_mut();
    let flags = SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_FULLMUTEX;
    let code = unsafe { sqlite3_open_v2(path.as_ptr(), &mut db, flags, ptr::null()) };
    let raw = NonNull::new(db).ok_or_else(|| Error::new("sqlite3_open_v2 returned null"))?;
    let conn = Connection { raw };

    if code == SQLITE_OK {
        Ok(conn)
    } else {
        let message = conn.error_message();
        drop(conn);
        Err(Error::new(message))
    }
}

fn configure(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode=WAL;
        PRAGMA busy_timeout=5000;
        PRAGMA foreign_keys=ON;
        "#,
    )
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS notes(
            path TEXT PRIMARY KEY,
            file_id TEXT NOT NULL,
            mtime INTEGER NOT NULL,
            content_hash TEXT NOT NULL,
            title TEXT,
            kind TEXT NOT NULL DEFAULT 'text'
        );

        CREATE TABLE IF NOT EXISTS chunks(
            id INTEGER PRIMARY KEY,
            note_path TEXT NOT NULL REFERENCES notes(path) ON DELETE CASCADE,
            parent_id INTEGER NULL REFERENCES chunks(id) ON DELETE CASCADE,
            level INTEGER NOT NULL,
            heading_path TEXT NOT NULL,
            ordinal INTEGER NOT NULL,
            chunk_stable_id TEXT NOT NULL UNIQUE,
            content_hash TEXT NOT NULL DEFAULT '',
            text TEXT NOT NULL
        );

        -- sqlite-vec fallback: brute-force cosine.
        -- The sandbox cannot fetch the sqlite-vec crate, so this table keeps the
        -- requested vec_chunks surface while storing 384-dim float32 vectors as BLOBs.
        CREATE TABLE IF NOT EXISTS vec_chunks(
            chunk_id INTEGER PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
            embedding BLOB NOT NULL CHECK(length(embedding) = 1536)
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

        CREATE TABLE IF NOT EXISTS links(
            source_path TEXT NOT NULL,
            target_path TEXT NOT NULL,
            kind TEXT NOT NULL,
            resolved INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS links_source_idx ON links(source_path);
        CREATE INDEX IF NOT EXISTS links_target_idx ON links(target_path);

        CREATE TABLE IF NOT EXISTS meta(
            k TEXT PRIMARY KEY,
            v TEXT NOT NULL
        );

        INSERT OR REPLACE INTO meta(k, v) VALUES ('schema_version', '2');
        INSERT OR IGNORE INTO meta(k, v) VALUES ('embedding_model', '');
        INSERT OR IGNORE INTO meta(k, v) VALUES ('embedding_dim', '384');
        INSERT OR IGNORE INTO meta(k, v) VALUES ('chunker_ver', '');
        INSERT OR IGNORE INTO meta(k, v) VALUES ('vault_id', '');
        "#,
    )?;
    ensure_column(
        conn,
        "notes",
        "kind",
        "ALTER TABLE notes ADD COLUMN kind TEXT NOT NULL DEFAULT 'text'",
    )?;
    ensure_column(
        conn,
        "chunks",
        "content_hash",
        "ALTER TABLE chunks ADD COLUMN content_hash TEXT NOT NULL DEFAULT ''",
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
    upsert_file(conn, path, file_id, mtime, content_hash, title, "text")
}

pub fn upsert_file(
    conn: &Connection,
    path: &str,
    file_id: &str,
    mtime: i64,
    content_hash: &str,
    title: Option<&str>,
    kind: &str,
) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO notes(path, file_id, mtime, content_hash, title, kind)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(path) DO UPDATE SET
            file_id = excluded.file_id,
            mtime = excluded.mtime,
            content_hash = excluded.content_hash,
            title = excluded.title,
            kind = excluded.kind
        "#,
        &[
            Bind::Text(path),
            Bind::Text(file_id),
            Bind::I64(mtime),
            Bind::Text(content_hash),
            Bind::OptionalText(title),
            Bind::Text(kind),
        ],
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
    insert_chunk_with_hash(
        conn,
        note_path,
        parent_id,
        level,
        heading_path,
        ordinal,
        chunk_stable_id,
        "",
        text,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn insert_chunk_with_hash(
    conn: &Connection,
    note_path: &str,
    parent_id: Option<i64>,
    level: i64,
    heading_path: &str,
    ordinal: i64,
    chunk_stable_id: &str,
    content_hash: &str,
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
            content_hash,
            text
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        &[
            Bind::Text(note_path),
            Bind::OptionalI64(parent_id),
            Bind::I64(level),
            Bind::Text(heading_path),
            Bind::I64(ordinal),
            Bind::Text(chunk_stable_id),
            Bind::Text(content_hash),
            Bind::Text(text),
        ],
    )?;

    Ok(unsafe { sqlite3_last_insert_rowid(conn.raw.as_ptr()) })
}

#[allow(clippy::too_many_arguments)]
pub fn upsert_chunk_with_hash(
    conn: &Connection,
    note_path: &str,
    parent_id: Option<i64>,
    level: i64,
    heading_path: &str,
    ordinal: i64,
    chunk_stable_id: &str,
    content_hash: &str,
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
            content_hash,
            text
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(chunk_stable_id) DO UPDATE SET
            note_path = excluded.note_path,
            parent_id = excluded.parent_id,
            level = excluded.level,
            heading_path = excluded.heading_path,
            ordinal = excluded.ordinal,
            content_hash = excluded.content_hash,
            text = excluded.text
        "#,
        &[
            Bind::Text(note_path),
            Bind::OptionalI64(parent_id),
            Bind::I64(level),
            Bind::Text(heading_path),
            Bind::I64(ordinal),
            Bind::Text(chunk_stable_id),
            Bind::Text(content_hash),
            Bind::Text(text),
        ],
    )?;

    chunk_id_by_stable_id(conn, chunk_stable_id)?
        .ok_or_else(|| Error::new("upserted chunk could not be found"))
}

pub fn insert_embedding(conn: &Connection, chunk_id: i64, embedding: &[f32]) -> Result<()> {
    validate_embedding(embedding)?;
    let embedding = normalize_embedding(embedding);
    let embedding = f32_blob(&embedding);

    conn.execute(
        "INSERT OR REPLACE INTO vec_chunks(chunk_id, embedding) VALUES (?1, ?2)",
        &[Bind::I64(chunk_id), Bind::Blob(&embedding)],
    )?;
    Ok(())
}

pub fn vec_search(conn: &Connection, query_vec: &[f32], k: usize) -> Result<Vec<(i64, f64)>> {
    validate_embedding(query_vec)?;
    if k == 0 {
        return Ok(Vec::new());
    }
    let query_vec = normalize_embedding(query_vec);
    let mut best: Vec<(i64, f32)> = Vec::with_capacity(k);

    conn.query(
        "SELECT chunk_id, embedding FROM vec_chunks",
        &[],
        |statement| {
            let chunk_id = unsafe { sqlite3_column_int64(statement.raw, 0) };
            let embedding = statement.column_blob(1)?;
            let vector = f32_blob_to_vec(embedding)?;
            let score = dot_product(&query_vec, &vector);
            if best.len() < k {
                best.push((chunk_id, score));
            } else if let Some((min_index, (_, min_score))) = best
                .iter()
                .enumerate()
                .min_by(|(_, left), (_, right)| left.1.total_cmp(&right.1))
            {
                if score > *min_score {
                    best[min_index] = (chunk_id, score);
                }
            }
            Ok(())
        },
    )?;

    best.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    Ok(best
        .into_iter()
        .map(|(chunk_id, score)| (chunk_id, f64::from(score)))
        .collect())
}

pub fn fts_search(conn: &Connection, query: &str, k: usize) -> Result<Vec<(i64, f64)>> {
    let mut matches = Vec::new();
    conn.query(
        r#"
        SELECT rowid, rank
        FROM fts_chunks
        WHERE fts_chunks MATCH ?1
        ORDER BY rank
        LIMIT ?2
        "#,
        &[Bind::Text(query), Bind::I64(k as i64)],
        |statement| {
            let chunk_id = unsafe { sqlite3_column_int64(statement.raw, 0) };
            let rank = unsafe { sqlite3_column_double(statement.raw, 1) };
            matches.push((chunk_id, rank));
            Ok(())
        },
    )?;
    Ok(matches)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkRecord {
    pub id: i64,
    pub note_path: String,
    pub heading_path: String,
    pub text: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct NoteRef {
    pub path: String,
    pub title: String,
}

pub fn note_content_hash(conn: &Connection, path: &str) -> Result<Option<String>> {
    let mut content_hash = None;
    conn.query(
        "SELECT content_hash FROM notes WHERE path = ?1",
        &[Bind::Text(path)],
        |statement| {
            content_hash = Some(statement.column_text(0)?);
            Ok(())
        },
    )?;
    Ok(content_hash)
}

pub fn delete_chunks_for_note(conn: &Connection, note_path: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM chunks WHERE note_path = ?1",
        &[Bind::Text(note_path)],
    )
}

pub fn chunk_id_by_stable_id(conn: &Connection, stable_id: &str) -> Result<Option<i64>> {
    let mut id = None;
    conn.query(
        "SELECT id FROM chunks WHERE chunk_stable_id = ?1",
        &[Bind::Text(stable_id)],
        |statement| {
            id = Some(unsafe { sqlite3_column_int64(statement.raw, 0) });
            Ok(())
        },
    )?;
    Ok(id)
}

pub fn embedding_exists_for_chunk(
    conn: &Connection,
    chunk_id: i64,
    content_hash: &str,
) -> Result<bool> {
    let mut exists = false;
    conn.query(
        r#"
        SELECT 1
        FROM chunks c
        JOIN vec_chunks v ON v.chunk_id = c.id
        WHERE c.id = ?1 AND c.content_hash = ?2
        LIMIT 1
        "#,
        &[Bind::I64(chunk_id), Bind::Text(content_hash)],
        |_statement| {
            exists = true;
            Ok(())
        },
    )?;
    Ok(exists)
}

pub fn list_chunk_stable_ids_for_note(conn: &Connection, note_path: &str) -> Result<Vec<String>> {
    let mut stable_ids = Vec::new();
    conn.query(
        "SELECT chunk_stable_id FROM chunks WHERE note_path = ?1",
        &[Bind::Text(note_path)],
        |statement| {
            stable_ids.push(statement.column_text(0)?);
            Ok(())
        },
    )?;
    Ok(stable_ids)
}

pub fn delete_chunk_by_stable_id(conn: &Connection, stable_id: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM chunks WHERE chunk_stable_id = ?1",
        &[Bind::Text(stable_id)],
    )
}

pub fn chunk_by_id(conn: &Connection, chunk_id: i64) -> Result<Option<ChunkRecord>> {
    let mut chunk = None;
    conn.query(
        r#"
        SELECT id, note_path, heading_path, text
        FROM chunks
        WHERE id = ?1
        "#,
        &[Bind::I64(chunk_id)],
        |statement| {
            chunk = Some(ChunkRecord {
                id: unsafe { sqlite3_column_int64(statement.raw, 0) },
                note_path: statement.column_text(1)?,
                heading_path: statement.column_text(2)?,
                text: statement.column_text(3)?,
            });
            Ok(())
        },
    )?;
    Ok(chunk)
}

pub fn chunk_meta(conn: &Connection, chunk_id: i64) -> Result<Option<(String, String, String)>> {
    Ok(chunk_by_id(conn, chunk_id)?.map(|chunk| (chunk.note_path, chunk.heading_path, chunk.text)))
}

pub fn list_notes(conn: &Connection) -> Result<Vec<NoteRef>> {
    let mut notes = Vec::new();
    conn.query(
        r#"
        SELECT path, COALESCE(NULLIF(title, ''), path) AS title
        FROM notes
        ORDER BY title COLLATE NOCASE, path COLLATE NOCASE
        "#,
        &[],
        |statement| {
            notes.push(NoteRef {
                path: statement.column_text(0)?,
                title: statement.column_text(1)?,
            });
            Ok(())
        },
    )?;
    Ok(notes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRef {
    pub path: String,
    pub title: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRef {
    pub source_path: String,
    pub target_path: String,
    pub kind: String,
    pub resolved: bool,
}

pub fn list_files(conn: &Connection) -> Result<Vec<FileRef>> {
    let mut files = Vec::new();
    conn.query(
        r#"
        SELECT path, COALESCE(NULLIF(title, ''), path) AS title, COALESCE(kind, 'text')
        FROM notes
        ORDER BY path COLLATE NOCASE
        "#,
        &[],
        |statement| {
            files.push(FileRef {
                path: statement.column_text(0)?,
                title: statement.column_text(1)?,
                kind: statement.column_text(2)?,
            });
            Ok(())
        },
    )?;
    Ok(files)
}

pub fn delete_links_for_source(conn: &Connection, source_path: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM links WHERE source_path = ?1",
        &[Bind::Text(source_path)],
    )
}

pub fn insert_link(
    conn: &Connection,
    source_path: &str,
    target_path: &str,
    kind: &str,
    resolved: bool,
) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO links(source_path, target_path, kind, resolved)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        &[
            Bind::Text(source_path),
            Bind::Text(target_path),
            Bind::Text(kind),
            Bind::I64(if resolved { 1 } else { 0 }),
        ],
    )?;
    Ok(())
}

pub fn list_links(conn: &Connection) -> Result<Vec<LinkRef>> {
    let mut links = Vec::new();
    conn.query(
        "SELECT source_path, target_path, kind, resolved FROM links ORDER BY source_path, target_path, kind",
        &[],
        |statement| {
            links.push(LinkRef {
                source_path: statement.column_text(0)?,
                target_path: statement.column_text(1)?,
                kind: statement.column_text(2)?,
                resolved: unsafe { sqlite3_column_int64(statement.raw, 3) } != 0,
            });
            Ok(())
        },
    )?;
    Ok(links)
}

pub fn meta_value(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut value = None;
    conn.query(
        "SELECT v FROM meta WHERE k = ?1",
        &[Bind::Text(key)],
        |statement| {
            value = Some(statement.column_text(0)?);
            Ok(())
        },
    )?;
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheDep {
    pub note_path: String,
    pub chunk_stable_id: String,
    pub content_hash: String,
}

pub fn chunk_ai_meta(
    conn: &Connection,
    chunk_id: i64,
) -> Result<Option<(String, String, String, String, String)>> {
    let mut chunk = None;
    conn.query(
        r#"
        SELECT c.note_path, c.heading_path, c.text, c.chunk_stable_id, n.content_hash
        FROM chunks c
        JOIN notes n ON n.path = c.note_path
        WHERE c.id = ?1
        "#,
        &[Bind::I64(chunk_id)],
        |statement| {
            chunk = Some((
                statement.column_text(0)?,
                statement.column_text(1)?,
                statement.column_text(2)?,
                statement.column_text(3)?,
                statement.column_text(4)?,
            ));
            Ok(())
        },
    )?;
    Ok(chunk)
}

pub fn cache_get_valid(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut response = None;
    conn.query(
        "SELECT response FROM cache WHERE key = ?1",
        &[Bind::Text(key)],
        |statement| {
            response = Some(statement.column_text(0)?);
            Ok(())
        },
    )?;

    let Some(response) = response else {
        return Ok(None);
    };

    let mut valid = true;
    conn.query(
        r#"
        SELECT d.note_path, d.chunk_stable_id, d.content_hash, n.content_hash, c.note_path
        FROM cache_deps d
        LEFT JOIN notes n ON n.path = d.note_path
        LEFT JOIN chunks c ON c.chunk_stable_id = d.chunk_stable_id
        WHERE d.cache_key = ?1
        "#,
        &[Bind::Text(key)],
        |statement| {
            let dep_note_path = statement.column_text(0)?;
            let expected = statement.column_text(2)?;
            let actual = statement.column_text(3)?;
            let chunk_note_path = statement.column_text(4)?;
            if expected != actual || chunk_note_path != dep_note_path {
                valid = false;
            }
            Ok(())
        },
    )?;

    if valid {
        Ok(Some(response))
    } else {
        Ok(None)
    }
}

pub fn cache_put(
    conn: &Connection,
    key: &str,
    response: &str,
    model_ver: &str,
    deps: &[CacheDep],
) -> Result<()> {
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default();

    conn.execute(
        r#"
        INSERT INTO cache(key, response, model_ver, created_at)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(key) DO UPDATE SET
            response = excluded.response,
            model_ver = excluded.model_ver,
            created_at = excluded.created_at
        "#,
        &[
            Bind::Text(key),
            Bind::Text(response),
            Bind::Text(model_ver),
            Bind::I64(created_at),
        ],
    )?;
    conn.execute(
        "DELETE FROM cache_deps WHERE cache_key = ?1",
        &[Bind::Text(key)],
    )?;

    for dep in deps {
        conn.execute(
            r#"
            INSERT INTO cache_deps(cache_key, note_path, chunk_stable_id, content_hash)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            &[
                Bind::Text(key),
                Bind::Text(&dep.note_path),
                Bind::Text(&dep.chunk_stable_id),
                Bind::Text(&dep.content_hash),
            ],
        )?;
    }

    Ok(())
}

impl Connection {
    fn execute_batch(&self, sql: &str) -> Result<()> {
        let sql = cstring(sql)?;
        let mut errmsg = ptr::null_mut();
        let code = unsafe {
            sqlite3_exec(
                self.raw.as_ptr(),
                sql.as_ptr(),
                None,
                ptr::null_mut(),
                &mut errmsg,
            )
        };

        if code == SQLITE_OK {
            Ok(())
        } else if errmsg.is_null() {
            Err(Error::new(self.error_message()))
        } else {
            let message = unsafe { CStr::from_ptr(errmsg) }
                .to_string_lossy()
                .into_owned();
            unsafe {
                sqlite3_free(errmsg.cast());
            }
            Err(Error::new(message))
        }
    }

    fn execute(&self, sql: &str, params: &[Bind<'_>]) -> Result<usize> {
        let mut statement = self.prepare(sql)?;
        statement.bind_all(params)?;

        match unsafe { sqlite3_step(statement.raw) } {
            SQLITE_DONE => Ok(unsafe { sqlite3_changes(self.raw.as_ptr()) as usize }),
            code => Err(self.step_error(code)),
        }
    }

    fn query<F>(&self, sql: &str, params: &[Bind<'_>], mut visit: F) -> Result<()>
    where
        F: FnMut(&Statement<'_>) -> Result<()>,
    {
        let mut statement = self.prepare(sql)?;
        statement.bind_all(params)?;

        loop {
            match unsafe { sqlite3_step(statement.raw) } {
                SQLITE_ROW => visit(&statement)?,
                SQLITE_DONE => return Ok(()),
                code => return Err(self.step_error(code)),
            }
        }
    }

    fn prepare(&self, sql: &str) -> Result<Statement<'_>> {
        let sql = cstring(sql)?;
        let mut statement = ptr::null_mut();
        let code = unsafe {
            sqlite3_prepare_v2(
                self.raw.as_ptr(),
                sql.as_ptr(),
                -1,
                &mut statement,
                ptr::null_mut(),
            )
        };

        if code == SQLITE_OK {
            let raw = NonNull::new(statement)
                .ok_or_else(|| Error::new("sqlite3_prepare_v2 returned null"))?;
            Ok(Statement {
                conn: self,
                raw: raw.as_ptr(),
                text_params: Vec::new(),
                blob_params: Vec::new(),
            })
        } else {
            Err(self.step_error(code))
        }
    }

    fn step_error(&self, code: c_int) -> Error {
        Error::new(format!("sqlite error {code}: {}", self.error_message()))
    }

    fn error_message(&self) -> String {
        unsafe { CStr::from_ptr(sqlite3_errmsg(self.raw.as_ptr())) }
            .to_string_lossy()
            .into_owned()
    }
}

struct Statement<'conn> {
    conn: &'conn Connection,
    raw: *mut sqlite3_stmt,
    text_params: Vec<CString>,
    blob_params: Vec<Vec<u8>>,
}

impl Drop for Statement<'_> {
    fn drop(&mut self) {
        unsafe {
            sqlite3_finalize(self.raw);
        }
    }
}

impl Statement<'_> {
    fn bind_all(&mut self, params: &[Bind<'_>]) -> Result<()> {
        for (index, param) in params.iter().enumerate() {
            self.bind((index + 1) as c_int, param)?;
        }
        Ok(())
    }

    fn bind(&mut self, index: c_int, param: &Bind<'_>) -> Result<()> {
        let code = match param {
            Bind::I64(value) => unsafe { sqlite3_bind_int64(self.raw, index, *value) },
            Bind::OptionalI64(Some(value)) => unsafe {
                sqlite3_bind_int64(self.raw, index, *value)
            },
            Bind::OptionalI64(None) | Bind::OptionalText(None) => unsafe {
                sqlite3_bind_null(self.raw, index)
            },
            Bind::Text(value) => {
                self.text_params.push(cstring(value)?);
                let value = self.text_params.last().expect("just pushed text param");
                unsafe { sqlite3_bind_text(self.raw, index, value.as_ptr(), -1, None) }
            }
            Bind::OptionalText(Some(value)) => {
                self.text_params.push(cstring(value)?);
                let value = self.text_params.last().expect("just pushed text param");
                unsafe { sqlite3_bind_text(self.raw, index, value.as_ptr(), -1, None) }
            }
            Bind::Blob(value) => {
                self.blob_params.push(value.to_vec());
                let value = self.blob_params.last().expect("just pushed blob param");
                unsafe {
                    sqlite3_bind_blob(
                        self.raw,
                        index,
                        value.as_ptr().cast(),
                        value.len() as c_int,
                        None,
                    )
                }
            }
        };

        if code == SQLITE_OK {
            Ok(())
        } else {
            Err(self.conn.step_error(code))
        }
    }

    fn column_text(&self, index: c_int) -> Result<String> {
        let text = unsafe { sqlite3_column_text(self.raw, index) };
        if text.is_null() {
            Ok(String::new())
        } else {
            let text = unsafe { CStr::from_ptr(text.cast()) };
            Ok(text.to_string_lossy().into_owned())
        }
    }

    fn column_blob(&self, index: c_int) -> Result<&[u8]> {
        let blob = unsafe { sqlite3_column_blob(self.raw, index) };
        let len = unsafe { sqlite3_column_bytes(self.raw, index) };
        if len < 0 {
            return Err(Error::new("negative blob length returned by SQLite"));
        }
        if blob.is_null() && len == 0 {
            Ok(&[])
        } else if blob.is_null() {
            Err(Error::new("SQLite returned null blob with non-zero length"))
        } else {
            Ok(unsafe { std::slice::from_raw_parts(blob.cast(), len as usize) })
        }
    }
}

enum Bind<'a> {
    I64(i64),
    OptionalI64(Option<i64>),
    Text(&'a str),
    OptionalText(Option<&'a str>),
    Blob(&'a [u8]),
}

fn validate_embedding(embedding: &[f32]) -> Result<()> {
    if embedding.len() == EMBEDDING_DIM {
        Ok(())
    } else {
        Err(Error::new(format!(
            "embedding must have {EMBEDDING_DIM} dimensions, got {}",
            embedding.len()
        )))
    }
}

fn ensure_column(conn: &Connection, table: &str, column: &str, alter_sql: &str) -> Result<()> {
    let mut found = false;
    let sql = format!("PRAGMA table_info({table})");
    conn.query(&sql, &[], |statement| {
        if statement.column_text(1)? == column {
            found = true;
        }
        Ok(())
    })?;

    if found {
        Ok(())
    } else {
        conn.execute_batch(alter_sql)
    }
}

fn f32_blob(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<f32>());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn normalize_embedding(embedding: &[f32]) -> Vec<f32> {
    let mut normalized = embedding.to_vec();
    let norm = normalized
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();

    if norm == 0.0 {
        normalized.fill(0.0);
        normalized[0] = 1.0;
        return normalized;
    }

    for value in &mut normalized {
        *value /= norm;
    }
    normalized
}

fn f32_blob_to_vec(blob: &[u8]) -> Result<Vec<f32>> {
    if blob.len() != EMBEDDING_BYTES {
        return Err(Error::new(format!(
            "embedding blob must have {EMBEDDING_BYTES} bytes, got {}",
            blob.len()
        )));
    }

    Ok(blob
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

fn dot_product(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

#[cfg(test)]
fn cosine_distance(left: &[f32], right: &[f32]) -> f64 {
    let mut dot = 0.0_f64;
    let mut left_norm = 0.0_f64;
    let mut right_norm = 0.0_f64;

    for (left, right) in left.iter().zip(right) {
        let left = *left as f64;
        let right = *right as f64;
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
    }

    if left_norm == 0.0 || right_norm == 0.0 {
        f64::INFINITY
    } else {
        let distance = 1.0 - (dot / (left_norm.sqrt() * right_norm.sqrt()));
        if distance.abs() < 1e-12 {
            0.0
        } else {
            distance
        }
    }
}

fn cstring(value: &str) -> Result<CString> {
    CString::new(value).map_err(|_| Error::new("string contains nul byte"))
}

fn cstring_from_path(path: &Path) -> Result<CString> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| Error::new("path contains nul byte"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_blob_uses_four_bytes_per_float() {
        assert_eq!(f32_blob(&[1.0, 2.0]).len(), 8);
    }

    #[test]
    fn cosine_distance_is_zero_for_same_vector() {
        assert_eq!(cosine_distance(&[1.0, 2.0], &[1.0, 2.0]), 0.0);
    }
}
