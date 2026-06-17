use crate::db;
use crate::embed::Embedder;
use crate::graph::{self, GraphData};
use crate::markdown;
use crate::search;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

pub struct Indexer {
    conn: db::Connection,
    embedder: Box<dyn Embedder>,
    chunker_ver: u32,
    graph_notes: Vec<(String, Vec<String>, String)>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct IndexStats {
    pub notes: usize,
    pub chunks: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SearchHit {
    pub note_path: String,
    pub heading_path: String,
    pub snippet: String,
    pub score: f64,
}

impl Indexer {
    pub fn new(conn: db::Connection, embedder: Box<dyn Embedder>, chunker_ver: u32) -> Self {
        Self {
            conn,
            embedder,
            chunker_ver,
            graph_notes: Vec::new(),
        }
    }

    pub fn conn(&self) -> &db::Connection {
        &self.conn
    }

    pub fn index_vault(&mut self, root: &Path) -> Result<IndexStats, String> {
        if self.embedder.dim() != db::EMBEDDING_DIM {
            return Err(format!(
                "embedder dimension mismatch: expected {}, got {}",
                db::EMBEDDING_DIM,
                self.embedder.dim()
            ));
        }

        let mut stats = IndexStats::default();
        let mut graph_notes = Vec::new();
        let mut markdown_files = markdown_files(root)?;

        markdown_files.sort();

        for path in markdown_files {
            let content = fs::read_to_string(&path)
                .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
            let parsed = markdown::parse(&content);
            let note_path = path.to_string_lossy().into_owned();
            let content_hash = sha256_hex(content.as_bytes());
            graph_notes.push((
                note_path.clone(),
                parsed.wikilinks.clone(),
                parsed.title.clone(),
            ));
            stats.notes += 1;

            if db::note_content_hash(&self.conn, &note_path)
                .map_err(|err| err.to_string())?
                .as_deref()
                == Some(content_hash.as_str())
            {
                stats.skipped += 1;
                continue;
            }

            let metadata = fs::metadata(&path)
                .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
            let file_id = file_id(&path, &metadata);
            let mtime = metadata
                .modified()
                .ok()
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or_default();

            db::upsert_note(
                &self.conn,
                &note_path,
                &file_id,
                mtime,
                &content_hash,
                Some(&parsed.title),
            )
            .map_err(|err| err.to_string())?;
            db::delete_chunks_for_note(&self.conn, &note_path).map_err(|err| err.to_string())?;

            let mut chunk_ids = HashMap::new();
            for chunk in parsed.chunks {
                let parent_id = chunk
                    .parent_ordinal
                    .and_then(|ordinal| chunk_ids.get(&ordinal).copied());
                let stable_id = markdown::chunk_stable_id(
                    &file_id,
                    &chunk.heading_path,
                    chunk.ordinal,
                    self.chunker_ver,
                );
                let chunk_id = db::insert_chunk(
                    &self.conn,
                    &note_path,
                    parent_id,
                    i64::from(chunk.level),
                    &chunk.heading_path,
                    chunk.ordinal as i64,
                    &stable_id,
                    &chunk.text,
                )
                .map_err(|err| err.to_string())?;
                let embedding = self.embedder.embed(&chunk.text);
                db::insert_embedding(&self.conn, chunk_id, &embedding)
                    .map_err(|err| err.to_string())?;
                chunk_ids.insert(chunk.ordinal, chunk_id);
                stats.chunks += 1;
            }
        }

        self.graph_notes = graph_notes;
        Ok(stats)
    }

    pub fn graph(&self) -> GraphData {
        graph::build(&self.graph_notes)
    }

    pub fn search_fts(&self, query: &str, k: usize) -> Result<Vec<SearchHit>, String> {
        let matches = db::fts_search(&self.conn, query, k).map_err(|err| err.to_string())?;
        let mut hits = Vec::new();

        for (chunk_id, score) in matches {
            let Some(chunk) =
                db::chunk_by_id(&self.conn, chunk_id).map_err(|err| err.to_string())?
            else {
                continue;
            };
            hits.push(SearchHit {
                note_path: chunk.note_path,
                heading_path: chunk.heading_path,
                snippet: snippet(&chunk.text),
                score,
            });
        }

        Ok(hits)
    }

    pub fn search_hybrid(&self, query: &str, k: usize) -> Result<Vec<search::SearchHit>, String> {
        search::hybrid_search(&self.conn, self.embedder.as_ref(), query, k)
    }
}

fn markdown_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    if !root.exists() {
        return Err(format!("vault path does not exist: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("vault path is not a directory: {}", root.display()));
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|err| err.to_string())?;
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "md")
        {
            files.push(entry.path().to_path_buf());
        }
    }
    Ok(files)
}

fn snippet(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    const LIMIT: usize = 240;
    if collapsed.len() <= LIMIT {
        return collapsed;
    }

    let mut end = LIMIT;
    while !collapsed.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &collapsed[..end])
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut value = String::with_capacity(digest.len() * 2);
    for byte in digest {
        value.push_str(&format!("{byte:02x}"));
    }
    value
}

#[cfg(unix)]
fn file_id(_path: &Path, metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    format!("{}:{}", metadata.dev(), metadata.ino())
}

#[cfg(not(unix))]
fn file_id(path: &Path, _metadata: &fs::Metadata) -> String {
    path.to_string_lossy().into_owned()
}
