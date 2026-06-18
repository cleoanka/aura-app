use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkKind {
    Import,
    Include,
    Use,
    Wikilink,
    MdLink,
    Mention,
}

impl fmt::Display for LinkKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            LinkKind::Import => "Import",
            LinkKind::Include => "Include",
            LinkKind::Use => "Use",
            LinkKind::Wikilink => "Wikilink",
            LinkKind::MdLink => "MdLink",
            LinkKind::Mention => "Mention",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RawLink {
    pub target_hint: String,
    pub kind: LinkKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLink {
    pub target_path: String,
    pub resolved: bool,
}

pub fn extract_links(path: &Path, content: &str) -> Vec<RawLink> {
    let ext = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut links = Vec::new();
    match ext.as_str() {
        "py" | "pyi" => extract_python(content, &mut links),
        "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" => extract_c_includes(content, &mut links),
        "rs" => extract_rust(content, &mut links),
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => extract_js_ts(content, &mut links),
        "go" => extract_go(content, &mut links),
        _ if is_markdown_ext(&ext) => extract_markdown(content, &mut links),
        _ if filename == "dockerfile" => {}
        _ => {}
    }
    dedupe_links(links)
}

pub fn extract_links_with_mentions(
    path: &Path,
    content: &str,
    known_basenames: &HashMap<String, Vec<String>>,
) -> Vec<RawLink> {
    let mut links = extract_links(path, content);
    // audit #8: unicode-aware (is_alphanumeric + to_lowercase) → 'günlük.md'/'café.md' gibi
    // non-ASCII not adlarına düz-metin mention bağlantıları da kurulabilsin (Türkçe vault).
    let self_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_lowercase();

    let mut seen_mentions = HashSet::new();
    for token in content.split(|ch: char| !(ch.is_alphanumeric() || ch == '.' || ch == '_' || ch == '-'))
    {
        if token.is_empty() {
            continue;
        }
        let key = token.to_lowercase();
        if key == self_name {
            continue;
        }
        if known_basenames.contains_key(&key) && seen_mentions.insert(key) {
            links.push(RawLink {
                target_hint: token.to_string(),
                kind: LinkKind::Mention,
            });
        }
    }

    // audit #10: mention'lar seen_mentions ile, diğerleri extract_links içinde zaten dedup edilmiş;
    // kind'lar ayrık → ikinci dedupe_links saf no-op idi, kaldırıldı.
    links
}

pub fn known_basename_index(paths: &[PathBuf]) -> HashMap<String, Vec<String>> {
    let mut basenames: HashMap<String, Vec<String>> = HashMap::new();
    for path in paths {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        basenames
            .entry(name.to_lowercase())
            .or_default()
            .push(path.to_string_lossy().into_owned());
    }
    basenames
}

pub fn resolve_link(
    root: &Path,
    source_path: &Path,
    raw: &RawLink,
    project_files: &[PathBuf],
) -> Option<ResolvedLink> {
    let index = PathIndex::new(root, project_files);
    resolve_with_index(root, source_path, raw, &index)
}

pub fn resolve_links(
    root: &Path,
    source_path: &Path,
    raw_links: &[RawLink],
    project_files: &[PathBuf],
) -> Vec<(RawLink, ResolvedLink)> {
    let index = PathIndex::new(root, project_files);
    resolve_links_with_index(root, source_path, raw_links, &index)
}

/// PERF (codex #6): PathIndex'i ÖNCEDEN kurulmuş haliyle al — vault tarama başında
/// BİR KEZ kurulup her dosya için tekrar kullanılır (O(dosya²) → O(dosya)).
pub fn resolve_links_with_index(
    root: &Path,
    source_path: &Path,
    raw_links: &[RawLink],
    index: &PathIndex,
) -> Vec<(RawLink, ResolvedLink)> {
    raw_links
        .iter()
        .filter_map(|raw| {
            resolve_with_index(root, source_path, raw, index)
                .map(|resolved| (raw.clone(), resolved))
        })
        .collect()
}

fn resolve_with_index(
    root: &Path,
    source_path: &Path,
    raw: &RawLink,
    index: &PathIndex,
) -> Option<ResolvedLink> {
    let hint = raw.target_hint.trim();
    if hint.is_empty() {
        return None;
    }

    if let Some(path) = exact_candidates(root, source_path, hint)
        .into_iter()
        .find_map(|candidate| index.find_path(&candidate))
    {
        return Some(resolved(path, true));
    }

    if let Some(path) = index.find_basename(hint) {
        return Some(resolved(path, true));
    }

    if let Some(path) = heuristic_candidates(root, source_path, raw)
        .into_iter()
        .find_map(|candidate| index.find_path(&candidate))
    {
        return Some(resolved(path, true));
    }

    match raw.kind {
        LinkKind::Import | LinkKind::Include | LinkKind::Use if !looks_local(hint) => None,
        _ => Some(resolved(
            PathBuf::from(dangling_target(root, source_path, hint)),
            false,
        )),
    }
}

fn extract_python(content: &str, links: &mut Vec<RawLink>) {
    let import_re = Regex::new(r"(?m)^\s*import\s+(.+)$").expect("valid python import regex");
    let from_re = Regex::new(r"(?m)^\s*from\s+([A-Za-z_\.][\w\.]*)\s+import\s+")
        .expect("valid python from regex");

    for capture in import_re.captures_iter(content) {
        let Some(raw_modules) = capture.get(1) else {
            continue;
        };
        for module in raw_modules.as_str().split(',') {
            let module = module
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .trim_matches('.');
            if !module.is_empty() {
                links.push(RawLink {
                    target_hint: module.replace('.', "/"),
                    kind: LinkKind::Import,
                });
            }
        }
    }

    for capture in from_re.captures_iter(content) {
        let module = capture
            .get(1)
            .map(|value| value.as_str().trim_matches('.'))
            .unwrap_or_default();
        if !module.is_empty() {
            links.push(RawLink {
                target_hint: module.replace('.', "/"),
                kind: LinkKind::Import,
            });
        }
    }
}

fn extract_c_includes(content: &str, links: &mut Vec<RawLink>) {
    let include_re =
        Regex::new(r#"(?m)^\s*#\s*include\s*[<"]([^>"]+)[>"]"#).expect("valid include regex");
    for capture in include_re.captures_iter(content) {
        if let Some(target) = capture.get(1) {
            links.push(RawLink {
                target_hint: target.as_str().trim().to_string(),
                kind: LinkKind::Include,
            });
        }
    }
}

fn extract_rust(content: &str, links: &mut Vec<RawLink>) {
    let mod_re =
        Regex::new(r"(?m)^\s*(?:pub\s+)?mod\s+([A-Za-z_][\w]*)\s*;").expect("valid rust mod regex");
    let use_re = Regex::new(r"(?m)^\s*use\s+([^;]+);").expect("valid rust use regex");

    for capture in mod_re.captures_iter(content) {
        if let Some(name) = capture.get(1) {
            links.push(RawLink {
                target_hint: name.as_str().to_string(),
                kind: LinkKind::Use,
            });
        }
    }
    for capture in use_re.captures_iter(content) {
        if let Some(path) = capture.get(1) {
            links.push(RawLink {
                target_hint: path.as_str().trim().to_string(),
                kind: LinkKind::Use,
            });
        }
    }
}

fn extract_js_ts(content: &str, links: &mut Vec<RawLink>) {
    let import_export_re =
        Regex::new(r#"(?m)\b(?:import|export)\b(?:[^'";]*?\bfrom\s*)?["']([^"']+)["']"#)
            .expect("valid js import regex");
    let require_re =
        Regex::new(r#"\brequire\s*\(\s*["']([^"']+)["']\s*\)"#).expect("valid require regex");
    let dynamic_re =
        Regex::new(r#"\bimport\s*\(\s*["']([^"']+)["']\s*\)"#).expect("valid dynamic import regex");

    for capture in import_export_re
        .captures_iter(content)
        .chain(require_re.captures_iter(content))
        .chain(dynamic_re.captures_iter(content))
    {
        if let Some(spec) = capture.get(1) {
            links.push(RawLink {
                target_hint: spec.as_str().to_string(),
                kind: LinkKind::Import,
            });
        }
    }
}

fn extract_go(content: &str, links: &mut Vec<RawLink>) {
    let line_re = Regex::new(r#"(?m)^\s*import\s+(?:[._A-Za-z]\w*\s+)?"([^"]+)""#)
        .expect("valid go import regex");
    for capture in line_re.captures_iter(content) {
        if let Some(spec) = capture.get(1) {
            links.push(RawLink {
                target_hint: spec.as_str().to_string(),
                kind: LinkKind::Import,
            });
        }
    }

    let mut in_block = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import (") {
            in_block = true;
            continue;
        }
        if in_block && trimmed.starts_with(')') {
            in_block = false;
            continue;
        }
        if in_block {
            let quoted = trimmed
                .split('"')
                .nth(1)
                .filter(|value| !value.trim().is_empty());
            if let Some(spec) = quoted {
                links.push(RawLink {
                    target_hint: spec.to_string(),
                    kind: LinkKind::Import,
                });
            }
        }
    }
}

fn extract_markdown(content: &str, links: &mut Vec<RawLink>) {
    let wiki_re = Regex::new(r"\[\[([^\]]+)\]\]").expect("valid wikilink regex");
    let md_re = Regex::new(r"!?\[[^\]]*\]\(([^)]+)\)").expect("valid md link regex");
    let mut in_fence = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        for capture in wiki_re.captures_iter(line) {
            let Some(raw) = capture.get(1) else {
                continue;
            };
            let target = raw
                .as_str()
                .split('|')
                .next()
                .unwrap_or_default()
                .split('#')
                .next()
                .unwrap_or_default()
                .trim();
            if !target.is_empty() {
                links.push(RawLink {
                    target_hint: target.to_string(),
                    kind: LinkKind::Wikilink,
                });
            }
        }

        for capture in md_re.captures_iter(line) {
            let Some(raw) = capture.get(1) else {
                continue;
            };
            let target = raw.as_str().split('#').next().unwrap_or_default().trim();
            if target.is_empty() || target.contains("://") || target.starts_with('#') {
                continue;
            }
            links.push(RawLink {
                target_hint: target.to_string(),
                kind: LinkKind::MdLink,
            });
        }
    }
}

fn dedupe_links(links: Vec<RawLink>) -> Vec<RawLink> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for link in links {
        if seen.insert((link.target_hint.clone(), link.kind)) {
            deduped.push(link);
        }
    }
    deduped
}

fn is_markdown_ext(ext: &str) -> bool {
    matches!(ext, "md" | "markdown" | "mdx")
}

fn exact_candidates(root: &Path, source_path: &Path, hint: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let hint_path = Path::new(hint);
    if hint_path.is_absolute() {
        candidates.push(normalize_path(hint_path));
    } else {
        if let Some(parent) = source_path.parent() {
            candidates.push(normalize_path(&parent.join(hint_path)));
        }
        candidates.push(normalize_path(&root.join(hint_path)));
    }
    candidates
}

fn heuristic_candidates(root: &Path, source_path: &Path, raw: &RawLink) -> Vec<PathBuf> {
    match raw.kind {
        LinkKind::Import => import_candidates(root, source_path, &raw.target_hint),
        LinkKind::Use => rust_candidates(root, source_path, &raw.target_hint),
        LinkKind::MdLink | LinkKind::Wikilink => {
            markdown_candidates(root, source_path, &raw.target_hint)
        }
        LinkKind::Include | LinkKind::Mention => Vec::new(),
    }
}

fn import_candidates(root: &Path, source_path: &Path, hint: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let base_dirs: Vec<PathBuf> = if hint.starts_with("./") || hint.starts_with("../") {
        source_path
            .parent()
            .map(Path::to_path_buf)
            .into_iter()
            .collect()
    } else {
        vec![root.to_path_buf(), root.join("src")]
    };
    let rel = hint.trim_start_matches("./");

    for base in base_dirs {
        let base_target = normalize_path(&base.join(rel));
        push_with_extensions(
            &mut candidates,
            &base_target,
            &["py", "pyi", "ts", "tsx", "js", "jsx", "mjs", "cjs", "go"],
        );
        for ext in ["ts", "tsx", "js", "jsx", "mjs", "cjs"] {
            candidates.push(base_target.join(format!("index.{ext}")));
        }
        candidates.push(base_target.join("__init__.py"));
    }
    candidates
}

fn rust_candidates(root: &Path, source_path: &Path, hint: &str) -> Vec<PathBuf> {
    let cleaned = hint
        .replace("crate::", "")
        .replace("super::", "")
        .replace("self::", "")
        .replace("::", "/");
    let first = cleaned
        .split('{')
        .next()
        .unwrap_or(cleaned.as_str())
        .trim()
        .trim_end_matches('/');
    let mut candidates = Vec::new();
    let bases = [
        source_path.parent().unwrap_or(root).to_path_buf(),
        root.to_path_buf(),
        root.join("src"),
    ];

    let parts = first
        .split('/')
        .filter(|part| !part.is_empty() && part.chars().all(|ch| ch == '_' || ch.is_alphanumeric()))
        .collect::<Vec<_>>();
    for base in bases {
        for end in 1..=parts.len().max(1) {
            let rel = if parts.is_empty() {
                first.to_string()
            } else {
                parts[..end].join("/")
            };
            let target = normalize_path(&base.join(rel));
            candidates.push(target.with_extension("rs"));
            candidates.push(target.join("mod.rs"));
        }
    }
    candidates
}

fn markdown_candidates(root: &Path, source_path: &Path, hint: &str) -> Vec<PathBuf> {
    let mut candidates = exact_candidates(root, source_path, hint);
    if Path::new(hint).extension().is_none() {
        for base in exact_candidates(root, source_path, hint) {
            candidates.push(base.with_extension("md"));
            candidates.push(base.with_extension("markdown"));
            candidates.push(base.with_extension("mdx"));
        }
    }
    candidates
}

fn push_with_extensions(candidates: &mut Vec<PathBuf>, base: &Path, extensions: &[&str]) {
    if base.extension().is_some() {
        candidates.push(base.to_path_buf());
        return;
    }
    for ext in extensions {
        candidates.push(base.with_extension(ext));
    }
}

fn looks_local(hint: &str) -> bool {
    hint.starts_with("./")
        || hint.starts_with("../")
        || hint.starts_with('/')
        || hint.contains('/')
        || hint.contains('\\')
        || Path::new(hint).extension().is_some()
}

fn dangling_target(root: &Path, source_path: &Path, hint: &str) -> String {
    let hint_path = Path::new(hint);
    if hint_path.is_absolute() {
        return normalize_path(hint_path).to_string_lossy().into_owned();
    }
    if looks_local(hint) {
        return normalize_path(&source_path.parent().unwrap_or(root).join(hint_path))
            .to_string_lossy()
            .into_owned();
    }
    hint.to_string()
}

fn resolved(path: PathBuf, resolved: bool) -> ResolvedLink {
    ResolvedLink {
        target_path: path.to_string_lossy().into_owned(),
        resolved,
    }
}

pub struct PathIndex {
    paths: HashMap<String, PathBuf>,
    basenames: HashMap<String, Vec<PathBuf>>,
}

impl PathIndex {
    pub fn new(root: &Path, project_files: &[PathBuf]) -> Self {
        let mut paths = HashMap::new();
        let mut basenames: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for path in project_files {
            let normalized = normalize_path(path);
            paths.insert(path_key(&normalized), normalized.clone());
            if let Ok(relative) = normalized.strip_prefix(root) {
                paths.insert(path_key(relative), normalized.clone());
            }
            if let Some(name) = normalized.file_name().and_then(|name| name.to_str()) {
                basenames
                    .entry(name.to_ascii_lowercase())
                    .or_default()
                    .push(normalized.clone());
            }
            if let Some(stem) = normalized.file_stem().and_then(|stem| stem.to_str()) {
                basenames
                    .entry(stem.to_ascii_lowercase())
                    .or_default()
                    .push(normalized);
            }
        }
        Self { paths, basenames }
    }

    fn find_path(&self, path: &Path) -> Option<PathBuf> {
        self.paths.get(&path_key(&normalize_path(path))).cloned()
    }

    fn find_basename(&self, hint: &str) -> Option<PathBuf> {
        let key = Path::new(hint)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(hint)
            .to_ascii_lowercase();
        self.basenames
            .get(&key)
            .and_then(|paths| paths.first())
            .cloned()
    }
}

fn path_key(path: &Path) -> String {
    normalize_path(path).to_string_lossy().to_ascii_lowercase()
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}
