// Workspace (repo) semantiği: tek AKTİF kök. Yeni klasör seçmek eskisinin dosyalarını
// listeden/aramadan/graph'tan düşürür; "unut" DB izlerini tamamen siler.
use app_lib::db;
use app_lib::embed::StubEmbedder;
use app_lib::graph;
use app_lib::indexer::Indexer;
use app_lib::settings::{promote_root, MAX_RECENT_WORKSPACES};
use std::fs;
use std::path::PathBuf;

fn two_roots(name: &str) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let base = std::env::temp_dir().join(format!("aura-ws-{name}-{}", std::process::id()));
    if base.exists() {
        fs::remove_dir_all(&base).map_err(|e| e.to_string())?;
    }
    let old = base.join("old-repo");
    let new = base.join("new-repo");
    fs::create_dir_all(&old).map_err(|e| e.to_string())?;
    fs::create_dir_all(&new).map_err(|e| e.to_string())?;
    fs::write(old.join("eski.md"), "# Eski\n\noldunique zeta [[hedef]]\n")
        .map_err(|e| e.to_string())?;
    fs::write(new.join("yeni.md"), "# Yeni\n\nnewunique omega\n").map_err(|e| e.to_string())?;
    Ok((base, old, new))
}

#[test]
fn switching_workspace_hides_old_repo_everywhere() -> Result<(), String> {
    let (base, old, new) = two_roots("switch")?;

    let conn = db::open_in_memory().map_err(|e| e.to_string())?;
    let mut indexer = Indexer::new(conn, Box::new(StubEmbedder), 1);
    indexer.index_vault(&old)?;
    indexer.index_vault(&new)?;

    let active = new.to_string_lossy().into_owned();

    // list: yalnız aktif kökün notları
    let notes = db::list_notes_under(indexer.conn(), Some(&active)).map_err(|e| e.to_string())?;
    assert!(notes.iter().any(|n| n.path.ends_with("yeni.md")));
    assert!(
        !notes.iter().any(|n| n.path.ends_with("eski.md")),
        "eski repo'nun dosyası yeni workspace listesinde görünmemeli"
    );

    // list: workspace seçili değilse boş
    assert!(db::list_notes_under(indexer.conn(), None)
        .map_err(|e| e.to_string())?
        .is_empty());

    // FTS: eski kökün içeriği aktif kökte bulunmaz
    let hits = indexer.search_fts_in("oldunique", 5, Some(&active))?;
    assert!(hits.is_empty(), "eski repo içeriği aramada sızmamalı");
    let hits = indexer.search_fts_in("newunique", 5, Some(&active))?;
    assert_eq!(hits.len(), 1);

    // hybrid: aynı garanti
    let hits = indexer.search_hybrid_in("oldunique", 5, Some(&active))?;
    assert!(
        hits.is_empty(),
        "eski repo içeriği hybrid aramada sızmamalı"
    );
    let hits = indexer.search_hybrid_in("newunique", 5, Some(&active))?;
    assert!(!hits.is_empty());

    // graph: düğümler yalnız aktif kökten
    let g = graph::build_from_db_under(indexer.conn(), Some(&active)).map_err(|e| e.to_string())?;
    assert!(g.nodes.iter().any(|n| n.id.ends_with("yeni.md")));
    assert!(
        !g.nodes.iter().any(|n| n.id.ends_with("eski.md")),
        "eski repo graph'ta görünmemeli"
    );

    fs::remove_dir_all(&base).map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn forgetting_workspace_purges_all_traces() -> Result<(), String> {
    let (base, old, new) = two_roots("forget")?;

    let conn = db::open_in_memory().map_err(|e| e.to_string())?;
    let mut indexer = Indexer::new(conn, Box::new(StubEmbedder), 1);
    indexer.index_vault(&old)?;
    indexer.index_vault(&new)?;

    let deleted = db::delete_notes_under(indexer.conn(), &old.to_string_lossy())
        .map_err(|e| e.to_string())?;
    assert_eq!(deleted, 1, "eski kökte tam 1 not silinmeli");

    // Not, FTS ve link izleri gitti; yeni kök etkilenmedi
    assert!(!db::list_notes(indexer.conn())
        .map_err(|e| e.to_string())?
        .iter()
        .any(|n| n.path.ends_with("eski.md")));
    assert!(db::fts_search(indexer.conn(), "oldunique", 5)
        .map_err(|e| e.to_string())?
        .is_empty());
    assert!(db::list_notes(indexer.conn())
        .map_err(|e| e.to_string())?
        .iter()
        .any(|n| n.path.ends_with("yeni.md")));

    fs::remove_dir_all(&base).map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn path_under_root_is_component_wise() {
    assert!(db::path_under_root("/a/repo/x.md", "/a/repo"));
    assert!(db::path_under_root("/a/repo/alt/x.md", "/a/repo"));
    assert!(
        !db::path_under_root("/a/repo2/x.md", "/a/repo"),
        "string-prefix tuzağı: /a/repo2, /a/repo altında DEĞİL"
    );
    assert!(!db::path_under_root("/b/repo/x.md", "/a/repo"));
}

#[test]
fn promote_root_moves_to_front_dedups_and_caps() {
    let mut roots = vec!["/a".to_string(), "/b".to_string(), "/c".to_string()];

    // Mevcut girdiyi öne taşır (kopya bırakmaz)
    assert!(promote_root(&mut roots, "/b"));
    assert_eq!(roots, vec!["/b", "/a", "/c"]);

    // Zaten aktifse dokunmaz
    assert!(!promote_root(&mut roots, "/b"));
    assert_eq!(roots, vec!["/b", "/a", "/c"]);

    // Yeni girdi öne eklenir
    assert!(promote_root(&mut roots, "/d"));
    assert_eq!(roots.first().map(String::as_str), Some("/d"));

    // Liste sınırı korunur
    for i in 0..(MAX_RECENT_WORKSPACES + 5) {
        promote_root(&mut roots, &format!("/extra-{i}"));
    }
    assert_eq!(roots.len(), MAX_RECENT_WORKSPACES);
}
