// Gelişmiş agentic-retrieval seam'i. Şimdilik: yerel Ollama query-planner (Faz 2)
// + çok-sorgulu birleşim. Hepsi default-OFF; advanced_retrieval.enabled=false iken
// bu modül hiç çağrılmaz (ai.rs eski yola gider).
use crate::lane0;
use crate::search::SearchHit;
use crate::settings::Settings;
use serde::Deserialize;
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
    if settings.local_gen.provider != "ollama" {
        return None;
    }
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
