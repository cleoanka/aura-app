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
        ("How do I open a vault?", "How can I select my notes folder?"),
        ("What is consensus mode?", "Explain how consensus works."),
        ("How do I use a local model?", "How can I run generation with Ollama locally?"),
    ];
    // Farklı niyet (tuzak) — threshold'u GEÇMEMELİ (false-positive=0 şart).
    let trap = [
        ("How does hybrid search rank results?", "How do I delete a note?"),
        ("What does the answer cache do?", "What does the graph view show?"),
        ("How do I add an API key?", "How do I change the theme?"),
        ("Explain the consensus mode.", "Explain the embedding model download."),
        ("How do I open a vault?", "How do I export my notes?"),
        ("How do I use a local model?", "How do I review my git diff?"),
    ];

    const TH: f32 = 0.96; // settings.semantic_cache_threshold default (96/100)

    let mut fp = 0;
    let mut max_trap = f32::MIN;
    for (a, b) in trap {
        let c = cosine(&q(a), &q(b));
        println!("TRAP  {c:.3}  | {a}  ||  {b}");
        max_trap = max_trap.max(c);
        if c >= TH {
            fp += 1;
        }
    }
    let mut recall = 0;
    let mut min_para = f32::MAX;
    for (a, b) in para {
        let c = cosine(&q(a), &q(b));
        println!("PARA  {c:.3}  | {a}  ||  {b}");
        min_para = min_para.min(c);
        if c >= TH {
            recall += 1;
        }
    }
    println!("=> false-positive={fp}  recall={recall}/{}", para.len());
    println!("=> max_trap={max_trap:.3}  min_para={min_para:.3}  (ayrım bandı)");
    if max_trap < min_para {
        // Güvenli aralığın ortası: FP=0 marjı + paraphrase'leri yakalar.
        let safe = ((max_trap + min_para) / 2.0 * 100.0).round() / 100.0;
        println!("=> ÖNERİLEN threshold ≈ {safe:.2} (FP=0, recall↑)");
    } else {
        println!("=> ayrım YOK (max_trap ≥ min_para) → 0.96 muhafazakâr kalsın");
    }

    assert_eq!(fp, 0, "Anayasa Madde 9: tuzak çiftlerde false-positive OLMAMALI (TH={TH})");
}
