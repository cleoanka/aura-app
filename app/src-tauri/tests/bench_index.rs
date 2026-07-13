//! El-ile koşulan mini benchmark (BENCHMARKS.md'yi beslemek için; CI'da koşmaz):
//!   cargo test --test bench_index -- --ignored --nocapture
//! Sentetik vault: 300 markdown + 60 kod dosyası. StubEmbedder (embedding maliyeti hariç) —
//! ölçülen şey tarama+parse+chunk+link+FTS yazımı ve arama gecikmesi.
use app_lib::db;
use app_lib::embed::StubEmbedder;
use app_lib::indexer::Indexer;
use std::fs;
use std::time::Instant;

#[test]
#[ignore]
fn bench_index_and_search_synthetic_vault() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("aura-bench-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(root.join("src")).map_err(|e| e.to_string())?;

    for i in 0..300 {
        let body = format!(
            "# Not {i}\n\nBu bir sentetik not. [[Not {}]] bağlantısı ve biraz metin.\n\n\
             ## Bölüm A\n\nlorem ipsum kelime{} arama testi.\n\n## Bölüm B\n\ndaha fazla içerik.\n",
            (i + 1) % 300,
            i % 37
        );
        fs::write(root.join(format!("not-{i:03}.md")), body).map_err(|e| e.to_string())?;
    }
    for i in 0..60 {
        let body = format!(
            "import os\n\ndef fn_{i}():\n    return {i}  # kelime{}\n",
            i % 37
        );
        fs::write(root.join("src").join(format!("mod_{i:02}.py")), body)
            .map_err(|e| e.to_string())?;
    }

    let conn = db::open_in_memory().map_err(|e| e.to_string())?;
    let mut indexer = Indexer::new(conn, Box::new(StubEmbedder), 1);

    let stats_cold = indexer.index_vault(&root)?;
    let stats_warm = indexer.index_vault(&root)?; // içerik değişmedi → skip yolu

    let t = Instant::now();
    let hits = indexer.search_hybrid("kelime7 arama", 10)?;
    let search_ms = t.elapsed().as_micros() as f64 / 1000.0;

    println!(
        "BENCH cold: {} dosya / {} chunk / {} ms · warm(no-op): {} ms · hybrid arama: {:.2} ms ({} hit)",
        stats_cold.notes,
        stats_cold.chunks,
        stats_cold.elapsed_ms,
        stats_warm.elapsed_ms,
        search_ms,
        hits.len()
    );

    fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    Ok(())
}
