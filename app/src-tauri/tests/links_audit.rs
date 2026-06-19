// audit #8 (unicode mention) + #9 (md-link dengeli parantez) regresyon testleri.
use app_lib::links::{self, LinkKind};
use std::path::{Path, PathBuf};

#[test]
fn md_link_with_balanced_parens_in_path_not_truncated() {
    // #9: hedef yolu '(' içerse de ilk ')'de kesilmemeli.
    let links = links::extract_links(Path::new("doc.md"), "[draft](My File (draft).md)\n");
    assert!(
        links
            .iter()
            .any(|l| matches!(l.kind, LinkKind::MdLink) && l.target_hint == "My File (draft).md"),
        "dengeli parantezli hedef tam yakalanmalı: {:?}",
        links.iter().map(|l| &l.target_hint).collect::<Vec<_>>()
    );
}

#[test]
fn mention_matches_non_ascii_basename() {
    // #8: 'günlük.md' gibi non-ASCII dosya adına düz-metin mention kurulabilmeli (Türkçe vault).
    let known = links::known_basename_index(&[PathBuf::from("/vault/günlük.md")]);
    let links = links::extract_links_with_mentions(
        Path::new("/vault/other.md"),
        "Bugün günlük.md dosyasına baktım.\n",
        &known,
    );
    assert!(
        links
            .iter()
            .any(|l| matches!(l.kind, LinkKind::Mention) && l.target_hint == "günlük.md"),
        "non-ASCII mention bulunmalı: {:?}",
        links.iter().map(|l| (&l.target_hint, &l.kind)).collect::<Vec<_>>()
    );
}

#[test]
fn plain_md_link_still_works() {
    // Regresyon: parantezsiz normal md link hâlâ çalışmalı.
    let links = links::extract_links(Path::new("doc.md"), "[guide](Guide.md)\n");
    assert!(links
        .iter()
        .any(|l| matches!(l.kind, LinkKind::MdLink) && l.target_hint == "Guide.md"));
}
