//! [C] Semantic-cache emniyet eval'i. Anayasa Madde 9: SIFIR yanlış-cevap.
//! Semantic cache ancak threshold'ta TUZAK çiftler hit vermiyorsa (false-positive=0)
//! güvenlidir. Gerçek e5 modeli gerektirir → `#[ignore]` (normal gate'i yavaşlatmaz).
//! Çalıştır: `cargo test --test semantic_cache_eval -- --ignored --nocapture`

use app_lib::embed::default_embedder;

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[test]
#[ignore = "gerçek e5 modeli gerektirir; `cargo test --test semantic_cache_eval -- --ignored`"]
fn semantic_threshold_has_zero_false_positives() {
    let e = default_embedder();
    let q = |s: &str| e.embed_query(s);

    // Aynı niyet (paraphrase) — ideal olarak threshold'u geçer (recall).
    let para = [
        ("How does hybrid search rank results?", "What is the ranking method in hybrid search?"),
        ("How do I add an API key?", "Where do I enter my Anthropic API key?"),
        ("What does the answer cache do?", "Explain how the answer cache works."),
    ];
    // Farklı niyet (tuzak) — threshold'u GEÇMEMELİ (false-positive=0 şart).
    let trap = [
        ("How does hybrid search rank results?", "How do I delete a note?"),
        ("What does the answer cache do?", "What does the graph view show?"),
        ("How do I add an API key?", "How do I change the theme?"),
        ("Explain the consensus mode.", "Explain the embedding model download."),
    ];

    const TH: f32 = 0.96; // settings.semantic_cache_threshold default (96/100)

    let mut fp = 0;
    for (a, b) in trap {
        let c = cosine(&q(a), &q(b));
        println!("TRAP  {c:.3}  | {a}  ||  {b}");
        if c >= TH {
            fp += 1;
        }
    }
    let mut recall = 0;
    for (a, b) in para {
        let c = cosine(&q(a), &q(b));
        println!("PARA  {c:.3}  | {a}  ||  {b}");
        if c >= TH {
            recall += 1;
        }
    }
    println!("=> false-positive={fp}  recall={recall}/{}", para.len());

    assert_eq!(fp, 0, "Anayasa Madde 9: tuzak çiftlerde false-positive OLMAMALI (TH={TH})");
}
