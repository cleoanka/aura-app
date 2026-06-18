// embed.rs (StubEmbedder: boyut, normalizasyon, determinizm) + graph.rs (düğüm/link/dangling).
use app_lib::embed::{Embedder, StubEmbedder};
use app_lib::graph;

#[test]
fn stub_embedder_dim_is_384_and_l2_normalized() {
    let e = StubEmbedder;
    assert_eq!(e.dim(), 384);
    let v = e.embed("indexer mutex thread safety");
    assert_eq!(v.len(), 384);
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-3, "L2 norm ~1 olmalı, ölçülen {norm}");
}

#[test]
fn stub_embedder_deterministic_and_distinct() {
    let e = StubEmbedder;
    assert_eq!(e.embed("hello world"), e.embed("hello world"), "aynı metin → aynı vektör");
    assert_ne!(
        e.embed("hello world"),
        e.embed("completely unrelated content here"),
        "farklı metin → farklı vektör"
    );
    // stub'ta passage/query == embed
    assert_eq!(e.embed_passage("x"), e.embed("x"));
    assert_eq!(e.embed_query("x"), e.embed("x"));
}

#[test]
fn stub_embed_passages_batch_equals_single() {
    // #7: embed_pending artık batch kullanıyor; default (stub) yolda batch == tek-tek olmalı.
    let e = StubEmbedder;
    let texts = vec!["alpha passage".to_string(), "beta different text".to_string()];
    let batch = e.embed_passages_batch(&texts);
    assert_eq!(batch.len(), 2);
    assert_eq!(batch[0], e.embed_passage("alpha passage"));
    assert_eq!(batch[1], e.embed_passage("beta different text"));
}

#[test]
fn graph_build_links_existing_notes() {
    let notes = vec![
        ("a.md".to_string(), vec!["b.md".to_string()], "A".to_string()),
        ("b.md".to_string(), vec![], "B".to_string()),
    ];
    let g = graph::build(&notes);
    assert_eq!(g.nodes.len(), 2, "iki not iki düğüm");
    assert!(g.nodes.iter().any(|n| n.id == "a.md" && n.title == "A"));
    assert!(
        g.links.iter().any(|l| l.source == "a.md" && l.target == "b.md"),
        "a→b linki olmalı: {:?}",
        g.links
    );
    assert!(g.nodes.iter().all(|n| !n.dangling), "var olan notlar dangling değil");
}

#[test]
fn graph_build_handles_missing_target() {
    let notes = vec![(
        "a.md".to_string(),
        vec!["ghost-note.md".to_string()],
        "A".to_string(),
    )];
    let g = graph::build(&notes);
    assert!(g.nodes.iter().any(|n| n.id == "a.md"));
    let referenced = g.nodes.iter().any(|n| n.dangling)
        || g.links.iter().any(|l| l.target.contains("ghost-note"));
    assert!(
        referenced,
        "eksik hedef dangling düğüm veya link olarak görünmeli: nodes={:?} links={:?}",
        g.nodes, g.links
    );
}

#[test]
fn graph_build_empty_is_empty() {
    let g = graph::build(&[]);
    assert!(g.nodes.is_empty() && g.links.is_empty());
}
