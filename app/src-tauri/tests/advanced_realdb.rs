// Gerçek DB self-test: advanced retrieval pipeline'ını (planner→multi-query→graph→rerank→parent)
// kullanıcının GERÇEK index.sqlite3'ünde çalıştırır. GUI/bulut gerekmez.
// Çalıştır: cargo test --test advanced_realdb -- --ignored --nocapture
use app_lib::db;
use app_lib::embed::default_embedder;
use app_lib::indexer::Indexer;
use app_lib::retrieval;
use app_lib::settings::Settings;
use std::collections::HashMap;

#[test]
#[ignore]
fn advanced_assemble_on_real_db() {
    let mut db_dir = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .expect("data dir");
    db_dir.push("aura-app");
    let db_path = db_dir.join("index.sqlite3");
    if !db_path.exists() {
        eprintln!("gerçek DB yok ({}), atlanıyor", db_path.display());
        return;
    }
    let conn = db::open(&db_path).expect("db open");
    let indexer = Indexer::new(conn, default_embedder(), 1);

    let mut settings = Settings::default();
    settings.advanced_retrieval.enabled = true;

    let query = "indexer ne işe yarar";
    let plan = retrieval::plan_query_local(&settings, query);
    match &plan {
        Some(p) => eprintln!(
            "PLANNER OK → canonical='{}', expansions={}, keywords={:?}",
            p.canonical,
            p.expansions.len(),
            p.keywords
        ),
        None => eprintln!("PLANNER None (ollama kapalı/parse fail → ham sorgu)"),
    }

    let hits = retrieval::assemble(&indexer, &settings, query, plan.as_ref()).expect("assemble");
    let mut via: HashMap<String, usize> = HashMap::new();
    for h in &hits {
        *via.entry(h.via.clone()).or_insert(0) += 1;
    }
    eprintln!("TOPLAM HIT: {} | via dağılımı: {:?}", hits.len(), via);
    for h in hits.iter().take(10) {
        let name = h.note_path.rsplit('/').next().unwrap_or(&h.note_path);
        eprintln!("  [{:>6}] {} > {}", h.via, name, h.heading_path);
    }
    assert!(!hits.is_empty(), "advanced retrieval BOŞ döndü");
}
