use app_lib::db;
use app_lib::embed::StubEmbedder;
use app_lib::indexer::Indexer;
use std::fs;

#[test]
fn indexes_vault_searches_and_builds_graph() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("aura-indexer-smoke-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|err| err.to_string())?;
    }
    fs::create_dir_all(&root).map_err(|err| err.to_string())?;

    fs::write(
        root.join("a.md"),
        "# Alpha\n\nAlpha body mentions [[Beta]], renderer.o, and contains pomegranateunique.\n",
    )
    .map_err(|err| err.to_string())?;
    fs::write(
        root.join("b.md"),
        "# Beta\n\n```text\n[[IgnoredDangling]]\n```\n\n## Details\n\nBeta section has nectarineunique.\n",
    )
    .map_err(|err| err.to_string())?;
    fs::write(root.join("c.md"), "# Gamma\n\nGamma has archiveunique.\n")
        .map_err(|err| err.to_string())?;
    fs::write(
        root.join("main.py"),
        "import utils\n\ndef run():\n    return utils.VALUE\n",
    )
    .map_err(|err| err.to_string())?;
    fs::write(root.join("utils.py"), "VALUE = 'kiwiunique'\n").map_err(|err| err.to_string())?;
    fs::write(root.join("renderer.o"), [0, 159, 146, 150]).map_err(|err| err.to_string())?;

    let conn = db::open_in_memory().map_err(|err| err.to_string())?;
    let mut indexer = Indexer::new(conn, Box::new(StubEmbedder), 1);

    let stats = indexer.index_vault(&root)?;
    assert_eq!(stats.notes, 6);
    assert!(stats.chunks >= 5);

    let fts_matches =
        db::fts_search(indexer.conn(), "pomegranateunique", 5).map_err(|err| err.to_string())?;
    assert!(!fts_matches.is_empty());
    let chunk = db::chunk_by_id(indexer.conn(), fts_matches[0].0)
        .map_err(|err| err.to_string())?
        .expect("fts result should resolve to a chunk");
    assert!(chunk.note_path.ends_with("a.md"));

    let graph = indexer.graph();
    let alpha_id = root.join("a.md").to_string_lossy().into_owned();
    let beta_id = root.join("b.md").to_string_lossy().into_owned();
    let main_id = root.join("main.py").to_string_lossy().into_owned();
    let utils_id = root.join("utils.py").to_string_lossy().into_owned();
    let renderer_id = root.join("renderer.o").to_string_lossy().into_owned();
    assert!(graph
        .links
        .iter()
        .any(|link| link.source == alpha_id && link.target == beta_id));
    assert!(graph
        .links
        .iter()
        .any(|link| link.source == main_id && link.target == utils_id && link.kind == "Import"));
    assert!(graph.links.iter().any(|link| {
        link.source == alpha_id && link.target == renderer_id && link.kind == "Mention"
    }));
    assert!(graph
        .nodes
        .iter()
        .any(|node| node.id == beta_id && !node.dangling));
    assert!(graph
        .nodes
        .iter()
        .any(|node| node.id == renderer_id && node.kind == "binary" && !node.dangling));
    assert!(!graph
        .nodes
        .iter()
        .any(|node| node.id == "IgnoredDangling" && node.dangling));

    let second = indexer.index_vault(&root)?;
    assert_eq!(second.skipped, 6);

    fs::remove_dir_all(&root).map_err(|err| err.to_string())?;
    Ok(())
}
