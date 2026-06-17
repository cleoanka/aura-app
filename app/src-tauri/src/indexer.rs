use crate::db;
use crate::embed::Embedder;
use crate::graph::{self, GraphData};
use crate::links;
use crate::markdown;
use crate::search;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

pub struct Indexer {
    conn: db::Connection,
    embedder: Box<dyn Embedder>,
    chunker_ver: u32,
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
        let root = root.to_path_buf();
        let mut project_files = project_files(&root)?;
        project_files.sort_by(|left, right| left.path.cmp(&right.path));
        let project_paths = project_files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();
        let known_basenames = links::known_basename_index(&project_paths);
        let title_aliases = title_aliases(&project_files);

        for project_file in project_files {
            let path = project_file.path;
            let note_path = path.to_string_lossy().into_owned();
            let metadata = fs::metadata(&path)
                .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
            let file_id = file_id(&path, &metadata);
            let mtime = mtime(&metadata);
            stats.notes += 1;

            if project_file.text_candidate && metadata.len() <= MAX_TEXT_BYTES {
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        let parsed = markdown::parse_project_text(&path, &content);
                        let content_hash = sha256_hex(content.as_bytes());
                        let unchanged = db::note_content_hash(&self.conn, &note_path)
                            .map_err(|err| err.to_string())?
                            .as_deref()
                            == Some(content_hash.as_str());

                        db::upsert_file(
                            &self.conn,
                            &note_path,
                            &file_id,
                            mtime,
                            &content_hash,
                            Some(&parsed.title),
                            "text",
                        )
                        .map_err(|err| err.to_string())?;
                        self.reindex_links(
                            &root,
                            &path,
                            &content,
                            &known_basenames,
                            &project_paths,
                            &title_aliases,
                        )?;

                        if unchanged {
                            stats.skipped += 1;
                            continue;
                        }

                        stats.chunks += self.reindex_chunks(&note_path, &file_id, parsed.chunks)?;
                    }
                    Err(_) => {
                        let content_hash = metadata_hash(&metadata);
                        self.register_file(&note_path, &file_id, mtime, &content_hash, "binary")?;
                        db::delete_links_for_source(&self.conn, &note_path)
                            .map_err(|err| err.to_string())?;
                        stats.skipped += 1;
                    }
                }
            } else {
                let content_hash = metadata_hash(&metadata);
                let kind = if project_file.text_candidate {
                    stats.skipped += 1;
                    "binary"
                } else {
                    project_file.kind
                };
                let unchanged = db::note_content_hash(&self.conn, &note_path)
                    .map_err(|err| err.to_string())?
                    .as_deref()
                    == Some(content_hash.as_str());
                self.register_file(&note_path, &file_id, mtime, &content_hash, kind)?;
                db::delete_links_for_source(&self.conn, &note_path)
                    .map_err(|err| err.to_string())?;
                if unchanged {
                    stats.skipped += 1;
                }
            }
        }

        Ok(stats)
    }

    fn register_file(
        &self,
        path: &str,
        file_id: &str,
        mtime: i64,
        content_hash: &str,
        kind: &str,
    ) -> Result<(), String> {
        db::upsert_file(&self.conn, path, file_id, mtime, content_hash, None, kind)
            .map_err(|err| err.to_string())
    }

    /// Embedding'i olmayan chunk'lardan en fazla `limit` tanesini embed eder.
    /// Arka planda tekrar tekrar çağrılır (0 dönene kadar). İndekslemeyi yavaşlatmaz.
    pub fn embed_pending(&mut self, limit: usize) -> Result<usize, String> {
        let pending =
            db::chunks_missing_embedding(&self.conn, limit as i64).map_err(|err| err.to_string())?;
        let count = pending.len();
        for (chunk_id, text) in pending {
            let embedding = self.embedder.embed_passage(&text);
            db::insert_embedding(&self.conn, chunk_id, &embedding).map_err(|err| err.to_string())?;
        }
        Ok(count)
    }

    fn reindex_links(
        &self,
        root: &Path,
        path: &Path,
        content: &str,
        known_basenames: &HashMap<String, Vec<String>>,
        project_paths: &[PathBuf],
        title_aliases: &HashMap<String, String>,
    ) -> Result<(), String> {
        let note_path = path.to_string_lossy().into_owned();
        db::delete_links_for_source(&self.conn, &note_path).map_err(|err| err.to_string())?;
        let raw_links = links::extract_links_with_mentions(path, content, known_basenames);
        let mut seen = HashSet::new();
        for (raw, mut resolved) in links::resolve_links(root, path, &raw_links, project_paths) {
            if !resolved.resolved {
                if let Some(title_target) = title_aliases.get(&link_key(&raw.target_hint)) {
                    resolved.target_path = title_target.clone();
                    resolved.resolved = true;
                }
            }
            if seen.insert((
                note_path.clone(),
                resolved.target_path.clone(),
                raw.kind.to_string(),
            )) {
                db::insert_link(
                    &self.conn,
                    &note_path,
                    &resolved.target_path,
                    &raw.kind.to_string(),
                    resolved.resolved,
                )
                .map_err(|err| err.to_string())?
            }
        }
        Ok(())
    }

    fn reindex_chunks(
        &self,
        note_path: &str,
        file_id: &str,
        chunks: Vec<markdown::Chunk>,
    ) -> Result<usize, String> {
        let old_stable_ids = db::list_chunk_stable_ids_for_note(&self.conn, note_path)
            .map_err(|err| err.to_string())?;
        let mut kept_stable_ids = HashSet::new();
        let mut chunk_ids = HashMap::new();
        let mut chunk_count = 0usize;

        for chunk in chunks {
            let parent_id = chunk
                .parent_ordinal
                .and_then(|ordinal| chunk_ids.get(&ordinal).copied());
            let stable_id = markdown::chunk_stable_id(
                file_id,
                &chunk.heading_path,
                chunk.ordinal,
                self.chunker_ver,
            );
            let chunk_hash = sha256_hex(chunk.text.as_bytes());
            let chunk_id = db::upsert_chunk_with_hash(
                &self.conn,
                note_path,
                parent_id,
                i64::from(chunk.level),
                &chunk.heading_path,
                chunk.ordinal as i64,
                &stable_id,
                &chunk_hash,
                &chunk.text,
            )
            .map_err(|err| err.to_string())?;

            // Embedding INLINE yapılmaz (yavaş, candle CPU): indeksleme hızlı kalsın,
            // dosyalar/graph/FTS anında gelsin. Vektörler embed_pending ile arka planda dolar.
            kept_stable_ids.insert(stable_id);
            chunk_ids.insert(chunk.ordinal, chunk_id);
            chunk_count += 1;
        }

        for stable_id in old_stable_ids {
            if !kept_stable_ids.contains(&stable_id) {
                db::delete_chunk_by_stable_id(&self.conn, &stable_id)
                    .map_err(|err| err.to_string())?;
            }
        }

        Ok(chunk_count)
    }

    pub fn graph(&self) -> GraphData {
        graph::build_from_db(&self.conn).unwrap_or_default()
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

const MAX_TEXT_BYTES: u64 = 1_500_000;

struct ProjectFile {
    path: PathBuf,
    text_candidate: bool,
    kind: &'static str,
}

fn project_files(root: &Path) -> Result<Vec<ProjectFile>, String> {
    if !root.exists() {
        return Err(format!("vault path does not exist: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("vault path is not a directory: {}", root.display()));
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !is_ignored_dir(entry.path()))
    {
        let entry = entry.map_err(|err| err.to_string())?;
        if entry.file_type().is_file() {
            let path = entry.path().to_path_buf();
            let text_candidate = is_text_project_file(&path);
            let kind = if text_candidate {
                "text"
            } else if is_asset_file(&path) {
                "asset"
            } else {
                "binary"
            };
            files.push(ProjectFile {
                path,
                text_candidate,
                kind,
            });
        }
    }
    Ok(files)
}

fn is_ignored_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                ".git"
                    | "node_modules"
                    | "target"
                    | "dist"
                    | "build"
                    | ".venv"
                    | "venv"
                    | "__pycache__"
                    | ".next"
                    | ".cache"
                    | ".turbo"
                    | "vendor"
                    | "Pods"
                    | ".idea"
                    | ".vscode"
                    | "coverage"
                    | ".gradle"
                    | ".mvn"
                    | "out"
                    | "bin"
                    | "obj"
            )
        })
}

fn is_text_project_file(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("dockerfile"))
    {
        return true;
    }

    let ext = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "md" | "markdown"
            | "mdx"
            | "txt"
            | "rst"
            | "org"
            | "py"
            | "pyi"
            | "rs"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "mjs"
            | "cjs"
            | "c"
            | "h"
            | "cc"
            | "cpp"
            | "cxx"
            | "hpp"
            | "hh"
            | "go"
            | "java"
            | "kt"
            | "kts"
            | "swift"
            | "rb"
            | "php"
            | "cs"
            | "scala"
            | "sh"
            | "bash"
            | "zsh"
            | "fish"
            | "lua"
            | "dart"
            | "r"
            | "jl"
            | "ex"
            | "exs"
            | "sql"
            | "vue"
            | "svelte"
            | "html"
            | "css"
            | "scss"
            | "sass"
            | "less"
            | "json"
            | "json5"
            | "toml"
            | "yaml"
            | "yml"
            | "ini"
            | "cfg"
            | "conf"
            | "xml"
            | "gradle"
            | "make"
            | "mk"
            | "proto"
            | "graphql"
    )
}

fn is_asset_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "pdf" | "ico" | "avif"
    )
}

fn title_aliases(project_files: &[ProjectFile]) -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    for file in project_files {
        if !file.text_candidate || !is_markdown_file(&file.path) {
            continue;
        }
        let Ok(metadata) = fs::metadata(&file.path) else {
            continue;
        };
        if metadata.len() > MAX_TEXT_BYTES {
            continue;
        }
        let Ok(content) = fs::read_to_string(&file.path) else {
            continue;
        };
        let parsed = markdown::parse(&content);
        if !parsed.title.trim().is_empty() {
            aliases.insert(
                link_key(&parsed.title),
                file.path.to_string_lossy().into_owned(),
            );
        }
    }
    aliases
}

fn is_markdown_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown" | "mdx")
}

fn link_key(value: &str) -> String {
    value
        .split('|')
        .next()
        .unwrap_or_default()
        .split('#')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
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

fn metadata_hash(metadata: &fs::Metadata) -> String {
    sha256_hex(format!("{}:{}", metadata.len(), mtime(metadata)).as_bytes())
}

fn mtime(metadata: &fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
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
