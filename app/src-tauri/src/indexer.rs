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

/// PERF #5: index_vault'u tek transaction'a sarar. Hata/erken-dönüşte Drop'ta ROLLBACK,
/// başarıda commit() ile COMMIT. SQLite autocommit'in INSERT başına fsync'ini önler.
struct TxGuard<'a> {
    conn: &'a db::Connection,
    committed: bool,
}

impl<'a> TxGuard<'a> {
    fn begin(conn: &'a db::Connection) -> Result<Self, String> {
        conn.begin_immediate().map_err(|err| err.to_string())?;
        Ok(Self {
            conn,
            committed: false,
        })
    }

    fn commit(mut self) -> Result<(), String> {
        self.conn.commit().map_err(|err| err.to_string())?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for TxGuard<'_> {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.conn.rollback();
        }
    }
}
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
    /// audit #1: bu turda diskte bulunmadığı için DB'den temizlenen (silinen) not sayısı.
    #[serde(default)]
    pub pruned: usize,
    /// Gözlemlenebilirlik (Döngü 5): bu indeksleme turunun süresi (ms). Geriye-uyumlu.
    #[serde(default)]
    pub elapsed_ms: u64,
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

        let started = std::time::Instant::now();
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
        // PERF (codex #6): PathIndex'i tarama başında BİR KEZ kur (per-dosya değil) → O(dosya)
        let path_index = links::PathIndex::new(&root, &project_paths);
        // PRUNE (audit #1): diskte GÖRÜLEN tüm yollar (stat hatasıyla atlananlar dahil) — sonra
        // DB'de olup diskte olmayan notlar temizlenir. Set diskteki varlıktan toplanır.
        let seen: std::collections::HashSet<String> = project_paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        // PERF (codex #5): tüm indekslemeyi TEK transaction'a sar → INSERT başına fsync yerine
        // tek commit (büyük vault'ta çok daha hızlı). Hata olursa TxGuard Drop'ta ROLLBACK.
        let tx = TxGuard::begin(&self.conn)?;

        for project_file in project_files {
            let path = project_file.path;
            let note_path = path.to_string_lossy().into_owned();
            // Dayanıklılık (audit #5): tarama↔stat arası dosya silinirse/erişilemezse (TOCTOU:
            // editör temp, .lock, FS hiccup) TÜM indekslemeyi düşürme — sadece o dosyayı atla
            // (read_to_string Err kolundaki davranışla tutarlı).
            let Ok(metadata) = fs::metadata(&path) else {
                stats.skipped += 1;
                continue;
            };
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
                            &path_index,
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

        // PRUNE (audit #1): diskte artık olmayan (silinen/yeniden-adlandırılan) notları temizle →
        // FTS/vektör/graph/cache'te orphan/stale kalmaz. SADECE bu root altındaki notlara dokun
        // (tek DB'ye birden çok vault indekslenebildiğinden).
        for db_path in db::all_note_paths(&self.conn).map_err(|err| err.to_string())? {
            if Path::new(&db_path).starts_with(&root) && !seen.contains(&db_path) {
                db::delete_note_fully(&self.conn, &db_path).map_err(|err| err.to_string())?;
                stats.pruned += 1;
            }
        }

        tx.commit()?;
        stats.elapsed_ms = started.elapsed().as_millis() as u64;
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
        if pending.is_empty() {
            return Ok(0);
        }
        let count = pending.len();
        // PERF (codex #7): tüm bekleyen chunk'ları TEK batch forward'da embed et (per-chunk değil).
        let texts: Vec<String> = pending.iter().map(|(_, text)| text.clone()).collect();
        let embeddings = self.embedder.embed_passages_batch(&texts);
        if embeddings.len() == pending.len() {
            for ((chunk_id, _), embedding) in pending.iter().zip(embeddings.iter()) {
                db::insert_embedding(&self.conn, *chunk_id, embedding)
                    .map_err(|err| err.to_string())?;
            }
        } else {
            // Sözleşme ihlali (batch eksik döndü): GÜVENLİ tek-tek → her chunk embed edilir,
            // count doğru kalır (codex robustness bulgusu).
            for (chunk_id, text) in &pending {
                let embedding = self.embedder.embed_passage(text);
                db::insert_embedding(&self.conn, *chunk_id, &embedding)
                    .map_err(|err| err.to_string())?;
            }
        }
        Ok(count)
    }

    fn reindex_links(
        &self,
        root: &Path,
        path: &Path,
        content: &str,
        known_basenames: &HashMap<String, Vec<String>>,
        path_index: &links::PathIndex,
        title_aliases: &HashMap<String, String>,
    ) -> Result<(), String> {
        let note_path = path.to_string_lossy().into_owned();
        db::delete_links_for_source(&self.conn, &note_path).map_err(|err| err.to_string())?;
        let raw_links = links::extract_links_with_mentions(path, content, known_basenames);
        let mut seen = HashSet::new();
        for (raw, mut resolved) in links::resolve_links_with_index(root, path, &raw_links, path_index) {
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
            // audit #2: yerinde düzenlemede stable_id aynı kalıp text değişebilir → eski hash'i oku.
            let prev_hash = db::chunk_content_hash(&self.conn, &stable_id)
                .map_err(|err| err.to_string())?;
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

            // İçerik değiştiyse (veya yeni chunk) eski embedding'i sil → embed_pending taze vektör
            // üretir; aksi halde vec_search kalıcı olarak ESKİ metnin vektörünü kullanırdı.
            if prev_hash.as_deref() != Some(chunk_hash.as_str()) {
                db::delete_embedding(&self.conn, chunk_id).map_err(|err| err.to_string())?;
            }

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

    // Built-in denylist + the vault's own top-level .gitignore (simple entries).
    let extra_ignored = gitignore_names(root);
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !is_ignored_path(entry.path(), &extra_ignored))
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

/// Built-in denylist OR a name listed in the vault's `.gitignore`. Applied to
/// every walked entry — pruning a directory skips its whole subtree, which is
/// what keeps black-hole folders (node_modules, target, …) out of the index.
fn is_ignored_path(path: &Path, extra: &HashSet<String>) -> bool {
    if is_ignored_dir(path) {
        return true;
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| extra.contains(name))
}

/// Read simple entries from the vault's top-level `.gitignore` AND `.auraignore`
/// (AURA-specific: exclude files from indexing without touching git). Only plain
/// names (optionally `name/` or `/name`) are honored; glob/path patterns are left
/// to the built-in denylist so matching stays correct and predictable.
fn gitignore_names(root: &Path) -> HashSet<String> {
    let mut names = HashSet::new();
    for file in [".gitignore", ".auraignore"] {
        let Ok(content) = fs::read_to_string(root.join(file)) else {
            continue;
        };
        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
                continue;
            }
            let name = line.trim_start_matches('/').trim_end_matches('/');
            if name.is_empty()
                || name.contains('/')
                || name.contains('*')
                || name.contains('?')
                || name.contains('[')
            {
                continue;
            }
            names.insert(name.to_string());
        }
    }
    names
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

#[cfg(test)]
mod tests {
    use super::{gitignore_names, is_ignored_dir, is_ignored_path, snippet, IndexStats};
    use std::collections::HashSet;
    use std::path::Path;

    #[test]
    fn snippet_collapses_whitespace_and_truncates_safely() {
        assert_eq!(snippet("  a   b\n c  "), "a b c"); // boşluk daraltma
        let long = "x".repeat(300);
        let s = snippet(&long);
        assert!(s.ends_with("..."), "uzun metin ... ile biter");
        assert!(s.len() <= 243, "240 + ...");
        // çok-baytlı sınırda panik olmamalı
        let _ = snippet(&"ç".repeat(300));
    }

    #[test]
    fn index_stats_serializes_with_snake_case_fields() {
        // Frontend `IndexStats` tipi bu alan adlarına (snake_case) bağlı.
        let json = serde_json::to_value(IndexStats::default()).unwrap();
        for key in ["notes", "chunks", "skipped", "pruned", "elapsed_ms"] {
            assert!(json.get(key).is_some(), "IndexStats.{key} serileşmeli");
        }
    }

    #[test]
    fn denylist_dirs_are_ignored() {
        for dir in [".git", "node_modules", "target", "dist", "__pycache__", ".venv"] {
            assert!(
                is_ignored_dir(Path::new("/vault").join(dir).as_path()),
                "{dir} denylist'te ignored olmalı"
            );
        }
        assert!(!is_ignored_dir(Path::new("/vault/notes")));
        assert!(!is_ignored_dir(Path::new("/vault/README.md")));
    }

    #[test]
    fn extra_gitignore_names_are_ignored_alongside_denylist() {
        let extra: HashSet<String> = ["secrets".to_string(), "build-out".to_string()]
            .into_iter()
            .collect();
        assert!(is_ignored_path(Path::new("/vault/secrets"), &extra));
        assert!(is_ignored_path(Path::new("/vault/sub/build-out"), &extra));
        // built-in denylist still applies even with an extra set present
        assert!(is_ignored_path(Path::new("/vault/.git"), &extra));
        assert!(!is_ignored_path(Path::new("/vault/keepme"), &extra));
    }

    #[test]
    fn gitignore_parser_keeps_only_simple_names() {
        let dir = std::env::temp_dir().join(format!("aura-gitignore-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(
            dir.join(".gitignore"),
            "# comment\n\nbuild/\n/out\n*.log\nnode_modules\n!keep\nsrc/generated\n",
        )
        .expect("write .gitignore");
        let names = gitignore_names(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert!(names.contains("build"), "trailing slash strip");
        assert!(names.contains("out"), "leading slash strip");
        assert!(names.contains("node_modules"));
        assert!(!names.contains("*.log"), "glob atlanmalı");
        assert!(!names.contains("keep"), "! negation atlanmalı");
        assert!(!names.contains("src/generated"), "path atlanmalı");
        assert!(!names.contains(""), "boş/comment satırı yok");
    }

    #[test]
    fn auraignore_is_unioned_with_gitignore() {
        let dir = std::env::temp_dir().join(format!("aura-auraignore-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join(".gitignore"), "node_modules\n").expect("gitignore");
        std::fs::write(dir.join(".auraignore"), "scratch\n# not\nprivate/\n").expect("auraignore");
        let names = gitignore_names(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(names.contains("node_modules"), ".gitignore girdisi");
        assert!(names.contains("scratch"), ".auraignore girdisi");
        assert!(names.contains("private"), ".auraignore trailing-slash");
    }

    #[test]
    fn gitignore_names_empty_when_no_file() {
        let dir = std::env::temp_dir().join(format!("aura-gitignore-none-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let names = gitignore_names(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(names.is_empty());
    }
}
