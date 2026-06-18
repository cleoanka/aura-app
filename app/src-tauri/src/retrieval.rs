// Gelişmiş agentic-retrieval seam'i. Şimdilik: yerel Ollama query-planner (Faz 2)
// + çok-sorgulu birleşim. Hepsi default-OFF; advanced_retrieval.enabled=false iken
// bu modül hiç çağrılmaz (ai.rs eski yola gider).
use crate::db;
use crate::indexer::Indexer;
use crate::lane0;
use crate::search::SearchHit;
use crate::settings::Settings;
use std::collections::HashSet;

/// Yerel modelin ürettiği sorgu planı.
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub canonical: String,
    pub expansions: Vec<String>,
    pub keywords: Vec<String>,
}

// Küçük model JSON şemasına tam uymayabilir (ör. expansions'ı [["..."]] iç içe verir).
// Bu yüzden serde_json::Value'ya parse edip stringleri DÜZLEŞTİREREK toplarız.
fn flatten_strings(value: Option<&serde_json::Value>, out: &mut Vec<String>) {
    match value {
        Some(serde_json::Value::String(s)) => {
            let s = s.trim();
            if !s.is_empty() {
                out.push(s.to_string());
            }
        }
        Some(serde_json::Value::Array(items)) => {
            for it in items {
                flatten_strings(Some(it), out);
            }
        }
        _ => {}
    }
}

const PLANNER_FALLBACK_MODEL: &str = "qwen2.5:3b";

/// Sorguyu YEREL Ollama modeliyle planla (yeniden yaz + genişlet). Bulut kotası YEMEZ.
/// Kapalı / Ollama yok / JSON bozuk → None döner; çağıran ham sorguya düşer (asla kırılmaz).
pub fn plan_query_local(settings: &Settings, query: &str) -> Option<QueryPlan> {
    let adv = &settings.advanced_retrieval;
    if !adv.enabled || !adv.planner_enabled {
        return None;
    }
    // Planner Lane-0 GENERATION provider'ına bağlı DEĞİL (o ayrı bir özellik).
    // Sadece Ollama erişilebilir olması yeter → kuruluysa planner çalışır.
    if !lane0::ollama_available(&settings.local_gen.ollama_url) {
        return None;
    }
    let model = if adv.planner_model.trim().is_empty() {
        PLANNER_FALLBACK_MODEL.to_string()
    } else {
        adv.planner_model.clone()
    };
    let prompt = format!(
        "You are a retrieval query planner for a notes+code knowledge base. \
Given the user's question, output ONLY strict minified JSON with this exact shape:\n\
{{\"canonical\":\"one clear rewrite of the question\",\"expansions\":[\"2-4 alternative phrasings or sub-questions\"],\"keywords\":[\"key technical terms\"]}}\n\
No prose, no markdown fences. Keep the user's language.\n\nQUESTION:\n{query}"
    );
    let raw = lane0::ollama_generate(&settings.local_gen.ollama_url, &model, &prompt).ok()?;
    let json = extract_json(&raw)?;
    let value: serde_json::Value = serde_json::from_str(&json).ok()?;
    let canonical = value
        .get("canonical")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let mut expansions = Vec::new();
    flatten_strings(value.get("expansions"), &mut expansions);
    expansions.truncate(4);
    let mut keywords = Vec::new();
    flatten_strings(value.get("keywords"), &mut keywords);
    keywords.truncate(8);
    if canonical.is_empty() && expansions.is_empty() && keywords.is_empty() {
        return None;
    }
    Some(QueryPlan {
        canonical,
        expansions,
        keywords,
    })
}

/// Modelin çıktısındaki ilk {...} JSON bloğunu ayıkla (bazen prose ekliyor).
fn extract_json(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end > start {
        Some(s[start..=end].to_string())
    } else {
        None
    }
}

/// Planlı sorgu listesi: orijinal + canonical + expansions (tekilleştirilmiş).
pub fn query_variants(query: &str, plan: Option<&QueryPlan>) -> Vec<String> {
    let mut out = vec![query.to_string()];
    if let Some(p) = plan {
        if !p.canonical.is_empty() && p.canonical != query {
            out.push(p.canonical.clone());
        }
        for e in &p.expansions {
            if !out.iter().any(|q| q == e) {
                out.push(e.clone());
            }
        }
    }
    out
}

/// Basit tokenizer: küçük harf, alfasayısal kelimeler (>=3 harf), tekil.
fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for raw in text.split(|c: char| !c.is_alphanumeric()) {
        let w = raw.to_lowercase();
        if w.chars().count() >= 3 && seen.insert(w.clone()) {
            out.push(w);
        }
    }
    out
}

fn overlap_ratio(terms: &[String], text_lower: &str) -> f64 {
    if terms.is_empty() {
        return 0.0;
    }
    let hits = terms.iter().filter(|t| text_lower.contains(t.as_str())).count();
    hits as f64 / terms.len() as f64
}

/// Faz 4: deterministik yerel reranking. Over-retrieve edilmiş adayları
/// base(hybrid skor) + lexical-overlap + heading-match + graph-boost ile skorla,
/// final_k'ya kırp. Bulut çağrısına SADECE en isabetli + az sayıda blok gider (token↓).
pub fn rerank(
    query: &str,
    keywords: &[String],
    hits: Vec<SearchHit>,
    final_k: usize,
) -> Vec<SearchHit> {
    let mut terms = tokenize(query);
    for k in keywords {
        for t in tokenize(k) {
            if !terms.contains(&t) {
                terms.push(t);
            }
        }
    }
    let max_score = hits
        .iter()
        .map(|h| h.score)
        .fold(0.0_f64, f64::max)
        .max(1e-9);
    let mut scored: Vec<(f64, SearchHit)> = hits
        .into_iter()
        .map(|h| {
            let base = (h.score / max_score).clamp(0.0, 1.0);
            let heading_l = h.heading_path.to_lowercase();
            let body_l = h.snippet.to_lowercase();
            let lex = overlap_ratio(&terms, &body_l);
            let head = overlap_ratio(&terms, &heading_l);
            let graph_boost = if h.via == "graph" { 1.0 } else { 0.0 };
            let s = 0.50 * base + 0.25 * lex + 0.15 * head + 0.10 * graph_boost;
            (s, h)
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
        .into_iter()
        .take(final_k)
        .map(|(s, mut h)| {
            h.score = s;
            h
        })
        .collect()
}

/// Çok-sorgulu hit gruplarını chunk_stable_id ile tekille; round-robin ile harmanla,
/// k'ya kırp (her sorgunun en iyileri öne gelsin).
pub fn dedup_merge(groups: Vec<Vec<SearchHit>>, k: usize) -> Vec<SearchHit> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let max_len = groups.iter().map(|g| g.len()).max().unwrap_or(0);
    for i in 0..max_len {
        for g in &groups {
            if let Some(h) = g.get(i) {
                if seen.insert(h.chunk_stable_id.clone()) {
                    out.push(h.clone());
                    if out.len() >= k {
                        return out;
                    }
                }
            }
        }
    }
    out
}

/// Faz 1-5 tek yerde: çok-sorgulu hybrid → bağlantı-grafı → rerank → parent pull-in.
/// SADECE advanced.enabled iken çağrılır. Bulut çağrısı YOK (sadece bağlam toplar).
/// Graph/parent hataları ölümcül DEĞİL; orijinal sorgu hatası ölümcüldür.
pub fn assemble(
    indexer: &Indexer,
    settings: &Settings,
    query: &str,
    plan: Option<&QueryPlan>,
) -> Result<Vec<SearchHit>, String> {
    let adv = &settings.advanced_retrieval;
    let final_k = (adv.final_k as usize).max(6);
    let cand_k = (adv.candidate_k as usize).max(final_k);

    // 1) Çok-sorgulu birleşim (over-retrieve)
    let variants = query_variants(query, plan);
    let mut groups = Vec::with_capacity(variants.len());
    for (i, q) in variants.iter().enumerate() {
        match indexer.search_hybrid(q, cand_k) {
            Ok(g) => groups.push(g),
            Err(err) if i == 0 => return Err(err),
            Err(_) => {}
        }
    }
    let merged = dedup_merge(groups, cand_k);

    // 2) Rerank → en alakalı final_k seed (graph/parent bunların ÜSTÜNE eklenir, elenmez)
    let kw = plan.map(|p| p.keywords.clone()).unwrap_or_default();
    let mut hits = if adv.rerank_enabled {
        rerank(query, &kw, merged, final_k)
    } else {
        let mut m = merged;
        m.truncate(final_k);
        m
    };

    // 3) Bağlantı-grafı: EN ALAKALI seed'lerin komşularını ekle (rezerve; rerank'e girmez,
    //    böylece lexical eşleşmese de link yüzünden dahil olanlar görünür kalır)
    if adv.graph_enabled {
        let mut seed_paths: Vec<String> = Vec::new();
        for h in &hits {
            if !seed_paths.contains(&h.note_path) {
                seed_paths.push(h.note_path.clone());
            }
            if seed_paths.len() >= adv.seed_k as usize {
                break;
            }
        }
        let neighbors = db::linked_note_neighbors(
            indexer.conn(),
            &seed_paths,
            adv.graph_hops as usize,
            adv.graph_neighbors_per_seed as usize,
        )
        .unwrap_or_default();
        let nb_chunks =
            db::representative_chunks_for_notes(indexer.conn(), &neighbors, 1).unwrap_or_default();
        let existing: HashSet<String> = hits.iter().map(|h| h.chunk_stable_id.clone()).collect();
        // Cap = seed×per-seed (tek seed tüm slotları yemesin, codex #3) AMA token için
        // toplam 2×final_k ile sınırla (bağlam şişmesin, kullanıcının token hedefi).
        let graph_cap = (seed_paths.len().max(1) * adv.graph_neighbors_per_seed as usize)
            .min(final_k * 2);
        let mut added = 0usize;
        for (note, heading, text, stable, hash) in nb_chunks {
            if added >= graph_cap {
                break;
            }
            if existing.contains(&stable) {
                continue;
            }
            hits.push(SearchHit {
                note_path: note,
                heading_path: heading,
                snippet: text.chars().take(400).collect(),
                chunk_stable_id: stable,
                content_hash: hash,
                score: 0.0,
                via: "graph".to_string(),
            });
            added += 1;
        }
    }

    // 4) Parent pull-in (üst başlık bölümü)
    if adv.parent_pull_in_enabled {
        let existing: HashSet<String> = hits.iter().map(|h| h.chunk_stable_id.clone()).collect();
        let mut pseen: HashSet<String> = HashSet::new();
        let mut parents: Vec<SearchHit> = Vec::new();
        let limit = hits.len();
        for h in &hits {
            if parents.len() >= limit {
                break;
            }
            if let Ok(Some((note, heading, text, stable, hash))) =
                db::parent_chunk_for(indexer.conn(), &h.chunk_stable_id)
            {
                if !existing.contains(&stable) && pseen.insert(stable.clone()) {
                    parents.push(SearchHit {
                        note_path: note,
                        heading_path: heading,
                        snippet: text.chars().take(600).collect(),
                        chunk_stable_id: stable,
                        content_hash: hash,
                        score: 0.0,
                        via: "parent".to_string(),
                    });
                }
            }
        }
        hits.extend(parents);
    }

    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(id: &str) -> SearchHit {
        SearchHit {
            note_path: format!("{id}.md"),
            heading_path: String::new(),
            snippet: id.to_string(),
            chunk_stable_id: id.to_string(),
            content_hash: String::new(),
            score: 1.0,
            via: "test".to_string(),
        }
    }

    #[test]
    fn dedup_merge_unions_and_caps() {
        let g1 = vec![hit("a"), hit("b")];
        let g2 = vec![hit("b"), hit("c")];
        let merged = dedup_merge(vec![g1, g2], 10);
        let ids: Vec<_> = merged.iter().map(|h| h.chunk_stable_id.clone()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]); // round-robin: a(g1), b(g2 i0)?, ...
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn rerank_prefers_relevant_over_graph_only() {
        let mut strong = hit("strong");
        strong.score = 0.9;
        strong.heading_path = "Indexer FULLMUTEX".to_string();
        strong.snippet = "the indexer uses fullmutex for thread safety".to_string();
        strong.via = "hybrid".to_string();
        let mut graph_only = hit("graph");
        graph_only.score = 0.0;
        graph_only.heading_path = "Unrelated".to_string();
        graph_only.snippet = "something about licensing".to_string();
        graph_only.via = "graph".to_string();
        let out = rerank(
            "why does the indexer use fullmutex",
            &[],
            vec![graph_only, strong],
            2,
        );
        assert_eq!(out[0].chunk_stable_id, "strong"); // alakalı olan öne gelir
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn query_variants_dedup() {
        let plan = QueryPlan {
            canonical: "what is x".to_string(),
            expansions: vec!["x meaning".to_string(), "what is x".to_string()],
            keywords: vec![],
        };
        let v = query_variants("x?", Some(&plan));
        assert!(v.contains(&"x?".to_string()));
        assert!(v.contains(&"what is x".to_string()));
        // canonical appears once even though an expansion duplicates it
        assert_eq!(v.iter().filter(|q| *q == "what is x").count(), 1);
    }
}
