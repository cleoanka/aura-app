//! Cache invalidation is kept in sync with file content hashes: the answer
//! cache only serves a hit while every dependency note still has the content
//! hash it had when the answer was produced. Editing a source file in place
//! changes its hash → the cached answer is invalidated. (The retrieval-set
//! fingerprint in `cache_key` handles new/removed files at a higher layer; see
//! commands/ai.rs.)

use app_lib::db::{self, CacheDep};

fn seed(conn: &db::Connection, note: &str, note_hash: &str) -> db::Result<()> {
    db::upsert_note(conn, note, "fid", 1, note_hash, Some("T"))?;
    db::insert_chunk(conn, note, None, 0, "T", 0, &format!("{note}#0"), "body")?;
    Ok(())
}

fn deps(note: &str, hash: &str) -> Vec<CacheDep> {
    vec![CacheDep {
        note_path: note.to_string(),
        chunk_stable_id: format!("{note}#0"),
        content_hash: hash.to_string(),
    }]
}

#[test]
fn cache_hits_while_source_unchanged() -> db::Result<()> {
    let conn = db::open_in_memory()?;
    seed(&conn, "a.md", "h1")?;
    db::cache_put(&conn, "key1", "cached answer", "model-v", &deps("a.md", "h1"))?;

    assert_eq!(
        db::cache_get_valid(&conn, "key1")?,
        Some("cached answer".to_string())
    );
    Ok(())
}

#[test]
fn cache_invalidates_when_source_file_edited() -> db::Result<()> {
    let conn = db::open_in_memory()?;
    seed(&conn, "a.md", "h1")?;
    db::cache_put(&conn, "key1", "cached answer", "model-v", &deps("a.md", "h1"))?;

    // User edits a.md in place → new content hash for the same note.
    db::upsert_note(&conn, "a.md", "fid", 2, "h2", Some("T"))?;

    assert_eq!(db::cache_get_valid(&conn, "key1")?, None);
    Ok(())
}

#[test]
fn missing_cache_key_is_a_miss() -> db::Result<()> {
    let conn = db::open_in_memory()?;
    assert_eq!(db::cache_get_valid(&conn, "never-written")?, None);
    Ok(())
}

#[test]
fn noop_reindex_preserves_cache() -> db::Result<()> {
    // Açılışta her vault yeniden indekslenir; içerik değişmediyse (yalnız mtime)
    // cache GEÇERSİZLEŞMEMELİ — "meşgul vault yine cache'ten faydalanır" garantisi.
    let conn = db::open_in_memory()?;
    seed(&conn, "a.md", "h1")?;
    db::cache_put(&conn, "k", "answer", "model-v", &deps("a.md", "h1"))?;

    // Re-index aynı içeriği yazar: mtime değişir ama content_hash aynı.
    db::upsert_note(&conn, "a.md", "fid", 999, "h1", Some("T"))?;

    assert_eq!(db::cache_get_valid(&conn, "k")?, Some("answer".to_string()));
    Ok(())
}

#[test]
fn deleting_dependency_note_invalidates_cache_not_validates_it() -> db::Result<()> {
    // audit C11 regresyonu: eskiden notes silinince cache_deps CASCADE ile yok oluyor,
    // dep'siz kalan cache girdisi "geçerli" sayılıp BAYAT cevap dönüyordu.
    let conn = db::open_in_memory()?;
    seed(&conn, "a.md", "h1")?;
    db::cache_put(&conn, "key1", "cached answer", "model-v", &deps("a.md", "h1"))?;

    db::delete_note_fully(&conn, "a.md")?;

    assert_eq!(
        db::cache_get_valid(&conn, "key1")?,
        None,
        "silinen kaynağın cache'i HIT dönmemeli (bayat cevap)"
    );
    Ok(())
}

#[test]
fn deleting_dependency_chunk_invalidates_cache() -> db::Result<()> {
    // audit C11: reindex'te kaybolan başlık/bölüm (chunk) de bağımlı cache'i düşürmeli.
    let conn = db::open_in_memory()?;
    seed(&conn, "a.md", "h1")?;
    db::cache_put(&conn, "key1", "cached answer", "model-v", &deps("a.md", "h1"))?;

    db::delete_chunk_by_stable_id(&conn, "a.md#0")?;

    assert_eq!(db::cache_get_valid(&conn, "key1")?, None);
    Ok(())
}
