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
    hybrid_search_in(conn, embedder, query, k, None)
}

/// Root-filtreli aramada aday derinliği bu tavana kadar 4×'lenerek artar (codex:
/// sabit k×6, çok-workspace'li DB'de aktif kökün sonuçlarını aç bırakabiliyordu).
const MAX_SEARCH_DEPTH: usize = 5000;

/// Root-filtreli hybrid arama (workspace semantiği): sonuçlar yalnız `root` altındaki
/// notlardan gelir. Filtre skorlamadan SONRA uygulanır; k dolmadıysa ve daha derinde
/// aday kalmış olabilirse limit 4×'lenerek yeniden denenir (progressive deepening).
pub fn hybrid_search_in(
    conn: &db::Connection,
    embedder: &dyn Embedder,
    query: &str,
    k: usize,
    root: Option<&str>,
) -> Result<Vec<SearchHit>, String> {
    if k == 0 {
        return Ok(Vec::new());
    }

    let query_embedding = embedder.embed_query(query);
    let mut search_limit = k.saturating_mul(if root.is_some() { 6 } else { 2 });
    loop {
        let (hits, exhausted) =
            hybrid_search_pass(conn, query, &query_embedding, k, root, search_limit)?;
        // Filtresiz yol tek geçiş (eski davranışla bit-aynı). Filtreli yolda: k dolduysa,
        // kaynaklar tükendiyse ya da tavana geldiysek dur; yoksa derinleş.
        if root.is_none() || hits.len() >= k || exhausted || search_limit >= MAX_SEARCH_DEPTH {
            return Ok(hits);
        }
        search_limit = search_limit.saturating_mul(4).min(MAX_SEARCH_DEPTH);
    }
}

fn hybrid_search_pass(
    conn: &db::Connection,
    query: &str,
    query_embedding: &[f32],
    k: usize,
    root: Option<&str>,
    search_limit: usize,
) -> Result<(Vec<SearchHit>, bool), String> {
    // FTS5 MATCH özel karakterlerde (C++, tırnak, ?, -foo) syntax hatası verebilir.
    // Bu ÖLÜMCÜL olmasın → boş FTS'e düş, vektör araması yine çalışsın (ask düşmez).
    let fts_ranked = db::fts_search(conn, query, search_limit).unwrap_or_default();
    let vec_ranked =
        db::vec_search(conn, query_embedding, search_limit).map_err(|err| err.to_string())?;
    // Her iki kaynak da limitin altında döndüyse daha derinde aday YOK.
    let exhausted = fts_ranked.len() < search_limit && vec_ranked.len() < search_limit;

    let fts_ids = fts_ranked
        .iter()
        .map(|(chunk_id, _rank)| *chunk_id)
        .collect::<Vec<_>>();
    let vec_ids = vec_ranked
        .iter()
        .map(|(chunk_id, _distance)| *chunk_id)
        .collect::<Vec<_>>();
    // Root filtresi varken fusion'ı derin tut (filtre sonrası k'yı doldurabilmek için);
    // filtresiz yol k ile aynı davranışta kalır (bit-aynı sonuç).
    let fused = rrf_fuse(&fts_ids, &vec_ids, if root.is_some() { search_limit } else { k });

    let fts_set = fts_ids.iter().copied().collect::<HashSet<_>>();
    let vec_set = vec_ids.iter().copied().collect::<HashSet<_>>();
    let mut hits = Vec::new();

    // PERF (codex #3): tüm metadata'yı TEK sorguda çek (N+1 değil), sonra fused sırasını koru.
    let fused_ids = fused.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    let meta = db::chunk_ai_meta_batch(conn, &fused_ids).map_err(|err| err.to_string())?;

    for (chunk_id, score) in fused {
        if hits.len() >= k {
            break;
        }
        let Some((note_path, heading_path, text, chunk_stable_id, content_hash)) =
            meta.get(&chunk_id).cloned()
        else {
            continue;
        };
        if let Some(root) = root {
            if !db::path_under_root(&note_path, root) {
                continue;
            }
        }
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

    Ok((hits, exhausted))
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
