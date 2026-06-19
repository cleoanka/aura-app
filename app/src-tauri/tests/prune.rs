// audit #1: diskten silinen not, yeniden indekslemede DB'den (FTS/list/graph) tamamen temizlenmeli.
use app_lib::db;
use app_lib::embed::StubEmbedder;
use app_lib::indexer::Indexer;
use std::fs;

#[test]
fn deleted_notes_are_pruned_from_all_subsystems() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("aura-prune-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    fs::write(root.join("a.md"), "# A\n\nalpha uniquekw links [[B]]\n").map_err(|e| e.to_string())?;
    fs::write(root.join("b.md"), "# B\n\nbeta keepmeword content\n").map_err(|e| e.to_string())?;

    let conn = db::open_in_memory().map_err(|e| e.to_string())?;
    let mut indexer = Indexer::new(conn, Box::new(StubEmbedder), 1);
    indexer.index_vault(&root)?;

    // b.md ilk indekste var: list + FTS
    assert!(db::list_notes(indexer.conn())
        .map_err(|e| e.to_string())?
        .iter()
        .any(|n| n.path.ends_with("b.md")));
    assert!(!db::fts_search(indexer.conn(), "keepmeword", 5)
        .map_err(|e| e.to_string())?
        .is_empty());

    // b.md'yi diskten sil → yeniden indeksle
    fs::remove_file(root.join("b.md")).map_err(|e| e.to_string())?;
    let stats = indexer.index_vault(&root)?;
    assert_eq!(stats.pruned, 1, "b.md tam olarak 1 not prune edilmeli");

    // artık HİÇBİR alt-sistemde yok
    assert!(
        !db::list_notes(indexer.conn())
            .map_err(|e| e.to_string())?
            .iter()
            .any(|n| n.path.ends_with("b.md")),
        "silinen not list_notes'ta kalmamalı"
    );
    assert!(
        db::fts_search(indexer.conn(), "keepmeword", 5)
            .map_err(|e| e.to_string())?
            .is_empty(),
        "silinen notun içeriği FTS'te kalmamalı"
    );
    let graph = indexer.graph();
    assert!(
        !graph
            .nodes
            .iter()
            .any(|n| n.id.ends_with("b.md") && !n.dangling),
        "silinen not graph'ta gerçek (dangling-olmayan) düğüm olarak kalmamalı"
    );

    // a.md hâlâ duruyor (yanlışlıkla budanmadı)
    assert!(db::list_notes(indexer.conn())
        .map_err(|e| e.to_string())?
        .iter()
        .any(|n| n.path.ends_with("a.md")));

    fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    Ok(())
}
