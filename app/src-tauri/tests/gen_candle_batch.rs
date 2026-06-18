// #7 candle batch DOĞRULAMASI: batch forward, tek-tek embed ile EŞDEĞER mi?
// Model indirir (~133MB) → yavaş + ağ → #[ignore]. Çalıştır:
//   cargo test --test gen_candle_batch -- --ignored --nocapture
#[cfg(feature = "candle")]
mod candle {
    use app_lib::embed::{CandleEmbedder, Embedder};

    #[test]
    #[ignore]
    fn batch_equals_single() {
        let embedder = match CandleEmbedder::new() {
            Ok(e) => e,
            Err(err) => {
                eprintln!("candle init başarısız (model indirilemedi?): {err} — test atlanıyor");
                return;
            }
        };
        let texts = vec![
            "passage about the indexer and hierarchical chunking".to_string(),
            "kısa".to_string(),
            "another unrelated sentence with different length here".to_string(),
        ];
        let batch = embedder.embed_passages_batch(&texts);
        assert_eq!(batch.len(), texts.len(), "batch boyutu");
        for (i, text) in texts.iter().enumerate() {
            assert_eq!(batch[i].len(), 384, "embedding boyutu");
            let single = embedder.embed_passage(text);
            let maxdiff = batch[i]
                .iter()
                .zip(&single)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            eprintln!("text {i}: batch-vs-single maxdiff = {maxdiff}");
            assert!(
                maxdiff < 1e-2,
                "batch[{i}] tek-tek embed'den sapıyor (maxdiff={maxdiff}) — batch yanlış"
            );
        }
        eprintln!("✓ candle batch == single (mask pad'leri dışlıyor, eşdeğer)");
    }
}
