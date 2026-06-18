// markdown.rs unit testleri: hiyerarşik chunking, başlık, wikilink, stable-id, kod-fence.
use app_lib::markdown::{self, chunk_stable_id};
use std::path::Path;

#[test]
fn parse_extracts_title_and_section_headings() {
    let md = "# Title\n\nintro\n\n## Section A\n\nbody a\n\n## Section B\n\nbody b\n";
    let p = markdown::parse(md);
    assert_eq!(p.title, "Title");
    assert!(!p.chunks.is_empty());
    assert!(p.chunks.iter().any(|c| c.heading_path.contains("Section A")));
    assert!(p.chunks.iter().any(|c| c.heading_path.contains("Section B")));

    // ordinaller benzersiz
    let mut ords: Vec<usize> = p.chunks.iter().map(|c| c.ordinal).collect();
    let n = ords.len();
    ords.sort_unstable();
    ords.dedup();
    assert_eq!(ords.len(), n, "ordinaller benzersiz olmalı");
}

#[test]
fn parse_no_headings_yields_chunks() {
    let p = markdown::parse("just some text\nwith no headings here\n");
    assert!(!p.chunks.is_empty());
    assert!(p.chunks.iter().any(|c| c.text.contains("no headings")));
}

#[test]
fn parse_extracts_wikilinks() {
    let p = markdown::parse("See [[Note One]] and also [[Note Two]] here.");
    assert!(
        p.wikilinks.iter().any(|w| w.contains("Note One")),
        "wikilinks: {:?}",
        p.wikilinks
    );
    assert!(p.wikilinks.iter().any(|w| w.contains("Note Two")));
}

#[test]
fn parse_ignores_headings_inside_code_fences() {
    let md = "# Real Title\n\n```\n# fake heading in code\n```\n\nbody text\n";
    let p = markdown::parse(md);
    assert_eq!(p.title, "Real Title");
    assert!(
        !p.chunks.iter().any(|c| c.heading_path.contains("fake heading")),
        "kod bloğundaki # başlık sayılmamalı"
    );
}

#[test]
fn parse_project_text_routes_by_extension() {
    let md = markdown::parse_project_text(Path::new("doc.md"), "# H\n\nbody");
    assert_eq!(md.title, "H");

    let code = markdown::parse_project_text(Path::new("mod.py"), "def f():\n    return 1\n");
    assert_eq!(code.title, "mod.py");
    assert!(!code.chunks.is_empty());

    let long: String = "line\n".repeat(120);
    let other = markdown::parse_project_text(Path::new("data.log"), &long);
    assert_eq!(other.title, "data.log");
    assert!(!other.chunks.is_empty(), "büyük metin chunk'lanmalı");
}

#[test]
fn chunk_stable_id_is_deterministic_and_sensitive() {
    let a = chunk_stable_id("file1", "A > B", 3, 1);
    assert_eq!(a, chunk_stable_id("file1", "A > B", 3, 1), "aynı girdi → aynı id");
    assert_eq!(a.len(), 64, "sha256 hex 64 karakter");
    assert_ne!(a, chunk_stable_id("file1", "A > B", 4, 1), "ordinal id'yi değiştirir");
    assert_ne!(a, chunk_stable_id("file1", "A > C", 3, 1), "heading_path id'yi değiştirir");
    assert_ne!(a, chunk_stable_id("file1", "A > B", 3, 2), "chunker_ver id'yi değiştirir");
    assert_ne!(a, chunk_stable_id("file2", "A > B", 3, 1), "file_id id'yi değiştirir");
}
