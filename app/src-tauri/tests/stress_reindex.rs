//! [B] Stress: eşzamanlı reindex ↔ search. App'te erişim `Mutex<Indexer>` ile
//! serialize edilir; bu test o yolu çok-thread altında döver — deadlock/panik/
//! bozulma olmamalı ve fırtına sonrası arama hâlâ çalışmalı.

use app_lib::db;
use app_lib::embed::StubEmbedder;
use app_lib::indexer::Indexer;
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn concurrent_reindex_and_search_is_stable() {
    let dir = std::env::temp_dir().join(format!("aura-stress-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp vault");
    for i in 0..8 {
        std::fs::write(
            dir.join(format!("note{i}.md")),
            format!("# Note {i}\n\nalpha beta gamma [[note{}]] govde {i}\n", (i + 1) % 8),
        )
        .expect("write note");
    }

    // Deterministik StubEmbedder: testi candle CPU-inference'ından bağımsız ve
    // hızlı tutar (amaç embedding kalitesi değil, Mutex/db yolunun stabilitesi).
    let conn = db::open_in_memory().expect("db");
    let indexer = Arc::new(Mutex::new(Indexer::new(conn, Box::new(StubEmbedder), 1)));
    indexer.lock().unwrap().index_vault(&dir).expect("ilk index");

    let mut handles = Vec::new();
    for t in 0..6 {
        let idx = Arc::clone(&indexer);
        let d = dir.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..10 {
                if t % 2 == 0 {
                    let _ = idx.lock().unwrap().index_vault(&d);
                } else {
                    let _ = idx.lock().unwrap().search_hybrid("alpha", 5);
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("thread panicked");
    }

    let hits = indexer
        .lock()
        .unwrap()
        .search_hybrid("alpha", 5)
        .expect("stress sonrası arama");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!hits.is_empty(), "stress sonrası arama sonuç döndürmeli");
}
