// Advanced retrieval pipeline'ının DETERMİNİSTİK testi (ollama/gerçek-DB gerekmez):
// graph-expansion lexical eşleşmeyen ama LİNK'li notu yüzeye çıkarır mı?
use app_lib::db;
use app_lib::embed::StubEmbedder;
use app_lib::indexer::Indexer;
use app_lib::retrieval;
use app_lib::settings::Settings;
use std::fs;

#[test]
fn assemble_surfaces_linked_note_via_graph() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("aura-assemble-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;

    // a.md sorguyla eşleşir + Beta'ya link verir; b.md sorguyla EŞLEŞMEZ (sadece link'le gelmeli).
    fs::write(
        root.join("a.md"),
        "# Alpha\n\nAlpha body has zephyrkeyword and links to [[Beta]].\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        root.join("b.md"),
        "# Beta\n\nBeta section is about unrelated plumbing internals.\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(root.join("c.md"), "# Gamma\n\nGamma is fully unrelated.\n")
        .map_err(|e| e.to_string())?;

    let conn = db::open_in_memory().map_err(|e| e.to_string())?;
    let mut indexer = Indexer::new(conn, Box::new(StubEmbedder), 1);
    indexer.index_vault(&root)?;

    let mut settings = Settings::default();
    settings.advanced_retrieval.enabled = true;
    // planner'ı çağırmıyoruz (None) → test ollama'dan bağımsız + deterministik.
    let hits = retrieval::assemble(&indexer, &settings, "zephyrkeyword", None)?;

    let a_id = root.join("a.md").to_string_lossy().into_owned();
    let b_id = root.join("b.md").to_string_lossy().into_owned();

    // a.md doğrudan (lexical) gelmeli
    assert!(
        hits.iter().any(|h| h.note_path == a_id),
        "seed a.md gelmeli: {:?}",
        hits.iter().map(|h| (&h.note_path, &h.via)).collect::<Vec<_>>()
    );
    // b.md SADECE link üzerinden (via=graph) gelmeli — lexical eşleşmiyor
    assert!(
        hits.iter().any(|h| h.note_path == b_id && h.via == "graph"),
        "b.md graph-expansion ile gelmeli: {:?}",
        hits.iter().map(|h| (&h.note_path, &h.via)).collect::<Vec<_>>()
    );

    fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn assemble_disabled_path_unaffected() -> Result<(), String> {
    // advanced kapalıyken assemble çağrılmaz; burada sadece search_hybrid'in çalıştığını doğrula.
    let root = std::env::temp_dir().join(format!("aura-assemble-off-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    fs::write(root.join("x.md"), "# X\n\nbananaunique content here.\n").map_err(|e| e.to_string())?;

    let conn = db::open_in_memory().map_err(|e| e.to_string())?;
    let mut indexer = Indexer::new(conn, Box::new(StubEmbedder), 1);
    indexer.index_vault(&root)?;

    let hits = indexer.search_hybrid("bananaunique", 6)?;
    assert!(hits.iter().any(|h| h.note_path.ends_with("x.md")));

    fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    Ok(())
}
