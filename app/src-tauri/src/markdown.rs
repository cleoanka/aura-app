use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNote {
    pub title: String,
    pub wikilinks: Vec<String>,
    pub chunks: Vec<Chunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub parent_ordinal: Option<usize>,
    pub level: u8,
    pub heading_path: String,
    pub ordinal: usize,
    pub text: String,
}

struct ChunkBuilder {
    parent_ordinal: Option<usize>,
    level: u8,
    heading_path: String,
    ordinal: usize,
    lines: Vec<String>,
}

pub fn parse(markdown: &str) -> ParsedNote {
    let heading_re = Regex::new(r"^(#{1,6})\s+(.+?)\s*#*\s*$").expect("valid heading regex");
    let title = find_title(markdown, &heading_re);
    let wikilinks = find_wikilinks(markdown);
    let chunks = parse_chunks(markdown, &heading_re, &title);

    ParsedNote {
        title,
        wikilinks,
        chunks,
    }
}

pub fn chunk_stable_id(
    file_id: &str,
    heading_path: &str,
    ordinal: usize,
    chunker_ver: u32,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(file_id.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(heading_path.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(ordinal.to_string().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(chunker_ver.to_string().as_bytes());
    hex_digest(hasher.finalize().as_slice())
}

fn find_title(markdown: &str, heading_re: &Regex) -> String {
    let mut in_fence = false;
    let mut first_non_empty = None;

    for line in markdown.lines() {
        let trimmed = line.trim();
        if first_non_empty.is_none() && !trimmed.is_empty() {
            first_non_empty = Some(trimmed.to_string());
        }

        if is_fence(trimmed) {
            in_fence = !in_fence;
            continue;
        }

        if !in_fence {
            if let Some((level, text)) = parse_heading(trimmed, heading_re) {
                if level == 1 {
                    return text;
                }
            }
        }
    }

    first_non_empty.unwrap_or_default()
}

fn find_wikilinks(markdown: &str) -> Vec<String> {
    let link_re = Regex::new(r"\[\[([^\]]+)\]\]").expect("valid wikilink regex");
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    let mut in_fence = false;

    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if is_fence(trimmed) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        for capture in link_re.captures_iter(line) {
            let Some(raw) = capture.get(1) else {
                continue;
            };
            let target = normalize_wikilink_target(raw.as_str());
            if !target.is_empty() && seen.insert(target.clone()) {
                links.push(target);
            }
        }
    }

    links
}

fn parse_chunks(markdown: &str, heading_re: &Regex, title: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut current: Option<ChunkBuilder> = None;
    let mut stack: Vec<(u8, String, usize)> = Vec::new();
    let mut in_fence = false;
    let mut next_ordinal = 0;

    for line in markdown.lines() {
        let trimmed = line.trim_start();
        let is_fence_line = is_fence(trimmed);

        if !in_fence {
            if let Some((level, heading_text)) = parse_heading(line.trim(), heading_re) {
                if (1..=3).contains(&level) {
                    finish_chunk(&mut chunks, current.take());

                    while stack
                        .last()
                        .is_some_and(|(stack_level, _, _)| *stack_level >= level)
                    {
                        stack.pop();
                    }

                    let parent_ordinal = stack.last().map(|(_, _, ordinal)| *ordinal);
                    let heading_path = if stack.is_empty() {
                        heading_text.clone()
                    } else {
                        let mut path: Vec<String> =
                            stack.iter().map(|(_, text, _)| text.clone()).collect();
                        path.push(heading_text.clone());
                        path.join(" > ")
                    };

                    let ordinal = next_ordinal;
                    next_ordinal += 1;
                    stack.push((level, heading_text.clone(), ordinal));
                    current = Some(ChunkBuilder {
                        parent_ordinal,
                        level,
                        heading_path,
                        ordinal,
                        lines: vec![heading_text],
                    });
                    continue;
                }
            }
        }

        if current.is_none() && !line.trim().is_empty() {
            current = Some(ChunkBuilder {
                parent_ordinal: None,
                level: 0,
                heading_path: title.to_string(),
                ordinal: next_ordinal,
                lines: Vec::new(),
            });
            next_ordinal += 1;
        }

        if let Some(chunk) = current.as_mut() {
            chunk.lines.push(line.to_string());
        }

        if is_fence_line {
            in_fence = !in_fence;
        }
    }

    finish_chunk(&mut chunks, current);

    if chunks.is_empty() && !markdown.trim().is_empty() {
        chunks.push(Chunk {
            parent_ordinal: None,
            level: 0,
            heading_path: title.to_string(),
            ordinal: 0,
            text: markdown.trim().to_string(),
        });
    }

    chunks
}

fn finish_chunk(chunks: &mut Vec<Chunk>, current: Option<ChunkBuilder>) {
    let Some(current) = current else {
        return;
    };
    let text = current.lines.join("\n").trim().to_string();
    if text.is_empty() {
        return;
    }

    chunks.push(Chunk {
        parent_ordinal: current.parent_ordinal,
        level: current.level,
        heading_path: current.heading_path,
        ordinal: current.ordinal,
        text,
    });
}

fn parse_heading(line: &str, heading_re: &Regex) -> Option<(u8, String)> {
    let captures = heading_re.captures(line)?;
    let hashes = captures.get(1)?;
    let text = captures.get(2)?.as_str().trim();
    Some((hashes.as_str().len() as u8, text.to_string()))
}

fn normalize_wikilink_target(raw: &str) -> String {
    raw.split('|')
        .next()
        .unwrap_or_default()
        .split('#')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn is_fence(trimmed_start: &str) -> bool {
    trimmed_start.starts_with("```") || trimmed_start.starts_with("~~~")
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push_str(&format!("{byte:02x}"));
    }
    value
}
