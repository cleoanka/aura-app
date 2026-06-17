use crate::db;
use crate::embed::Embedder;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SearchHit {
    pub note_path: String,
    pub heading_path: String,
    pub snippet: String,
    pub chunk_stable_id: String,
    pub content_hash: String,
    pub score: f64,
    pub via: String,
}

pub fn hybrid_search(
    conn: &db::Connection,
    embedder: &dyn Embedder,
    query: &str,
    k: usize,
) -> Result<Vec<SearchHit>, String> {
    if k == 0 {
        return Ok(Vec::new());
    }

    let search_limit = k.saturating_mul(2);
    let fts_ranked = db::fts_search(conn, query, search_limit).map_err(|err| err.to_string())?;
    let query_embedding = embedder.embed_query(query);
    let vec_ranked =
        db::vec_search(conn, &query_embedding, search_limit).map_err(|err| err.to_string())?;

    let fts_ids = fts_ranked
        .iter()
        .map(|(chunk_id, _rank)| *chunk_id)
        .collect::<Vec<_>>();
    let vec_ids = vec_ranked
        .iter()
        .map(|(chunk_id, _distance)| *chunk_id)
        .collect::<Vec<_>>();
    let fused = rrf_fuse(&fts_ids, &vec_ids, k);

    let fts_set = fts_ids.iter().copied().collect::<HashSet<_>>();
    let vec_set = vec_ids.iter().copied().collect::<HashSet<_>>();
    let mut hits = Vec::new();

    for (chunk_id, score) in fused {
        let Some((note_path, heading_path, text, chunk_stable_id, content_hash)) =
            db::chunk_ai_meta(conn, chunk_id).map_err(|err| err.to_string())?
        else {
            continue;
        };
        let via = match (fts_set.contains(&chunk_id), vec_set.contains(&chunk_id)) {
            (true, true) => "both",
            (true, false) => "fts",
            (false, true) => "vec",
            (false, false) => continue,
        };
        hits.push(SearchHit {
            note_path,
            heading_path,
            snippet: snippet(&text),
            chunk_stable_id,
            content_hash,
            score,
            via: via.to_string(),
        });
    }

    Ok(hits)
}

pub fn rrf_fuse(fts: &[i64], vec: &[i64], k: usize) -> Vec<(i64, f64)> {
    let mut scores = HashMap::<i64, f64>::new();

    for (position, chunk_id) in fts.iter().enumerate() {
        *scores.entry(*chunk_id).or_insert(0.0) += rrf_score(position);
    }
    for (position, chunk_id) in vec.iter().enumerate() {
        *scores.entry(*chunk_id).or_insert(0.0) += rrf_score(position);
    }

    let mut fused = scores.into_iter().collect::<Vec<_>>();
    fused.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    fused.truncate(k);
    fused
}

fn rrf_score(position: usize) -> f64 {
    1.0 / (60.0 + position as f64)
}

fn snippet(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    const LIMIT: usize = 200;
    if collapsed.len() <= LIMIT {
        return collapsed;
    }

    let mut end = LIMIT;
    while !collapsed.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &collapsed[..end])
}
