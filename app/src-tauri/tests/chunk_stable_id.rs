//! chunk_stable_id, incremental indexing ve cache_deps'in temelidir: aynı
//! girdiler aynı id'yi (yerinde düzenleme chunk'ı tutar), farklı girdiler farklı
//! id'yi vermeli (yeni/taşınan chunk yeni satır olur).

use app_lib::markdown::chunk_stable_id;

#[test]
fn stable_id_is_deterministic() {
    let a = chunk_stable_id("file-1", "Heading > Sub", 3, 1);
    let b = chunk_stable_id("file-1", "Heading > Sub", 3, 1);
    assert_eq!(a, b);
}

#[test]
fn stable_id_varies_with_every_input() {
    let base = chunk_stable_id("file-1", "Heading", 0, 1);
    assert_ne!(base, chunk_stable_id("file-2", "Heading", 0, 1), "file_id");
    assert_ne!(base, chunk_stable_id("file-1", "Other", 0, 1), "heading_path");
    assert_ne!(base, chunk_stable_id("file-1", "Heading", 1, 1), "ordinal");
    assert_ne!(base, chunk_stable_id("file-1", "Heading", 0, 2), "chunker_ver");
}
