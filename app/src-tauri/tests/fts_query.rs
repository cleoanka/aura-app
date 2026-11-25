use app_lib::db;
use app_lib::embed::StubEmbedder;
use app_lib::indexer::Indexer;
use std::fs;

// Ham kullanıcı metni FTS5 MATCH'e gidince noktalama syntax hatası veriyordu
// (`?`, `-`, `.`, `'`, `C++`) → arama ve Ask bağlamı boş dönüyordu.

#[test]
fn match_query_quotes_every_term_and_drops_punctuation_only_tokens() {
    assert_eq!(
        db::fts_match_query("what is alpha?").as_deref(),
        Some(r#""what" OR "is" OR "alpha?""#)
    );
    assert_eq!(db::fts_match_query(r#"a"b"#).as_deref(), Some(r#""a""b""#));
    assert_eq!(db::fts_match_query("  ?  - * "), None);
    assert_eq!(db::fts_match_query(""), None);
}

#[test]
fn punctuated_and_turkish_queries_match_without_syntax_errors() -> db::Result<()> {
    let conn = db::open_in_memory()?;
    let alpha = chunk(
        &conn,
        "alpha.md",
        "What is alpha in the auth-flow of note.md? It's documented here.",
    )?;
    let cpp = chunk(&conn, "cpp.md", "C++ templates don't compile on Fridays.")?;
    let turkish = chunk(
        &conn,
        "tr.md",
        "İndeksleyici nasıl çalışıyor? Şimdi değişen dosyaları tarıyor.",
    )?;

    let cases: &[(&str, i64)] = &[
        ("what is alpha?", alpha),
        ("auth-flow", alpha),
        ("note.md", alpha),
        ("it's", alpha),
        ("C++", cpp),
        ("don't", cpp),
        ("indeksleyici nasıl çalışıyor?", turkish),
        ("çalışıyor", turkish),
        ("ŞİMDİ", turkish),
        ("değişen dosyaları", turkish),
    ];
    for (query, expected) in cases {
        let hits = db::fts_search(&conn, query, 5)?;
        assert_eq!(
            hits.first().map(|hit| hit.0),
            Some(*expected),
            "query {query:?} → {hits:?}"
        );
    }

    // FTS5 operatörleri ve dengesiz tırnak artık sözdizimi değil, düz metin:
    // hata yok, enjeksiyon yok.
    for query in [
        r#""unbalanced"#,
        "NEAR(alpha templates)",
        "alpha AND",
        "-foo",
        "text:alpha",
        "alph*",
        "?",
        "",
    ] {
        db::fts_search(&conn, query, 5)?;
    }
    assert!(db::fts_search(&conn, "alph*", 5)?.is_empty());

    Ok(())
}

#[test]
fn indexer_fts_path_answers_punctuated_question() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("aura-fts-query-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|err| err.to_string())?;
    }
    fs::create_dir_all(&root).map_err(|err| err.to_string())?;
    fs::write(
        root.join("indexer.md"),
        "# Indexer\n\nIndexer nasıl çalışıyor: auth-flow dosyalarını tarar.\n",
    )
    .map_err(|err| err.to_string())?;

    let conn = db::open_in_memory().map_err(|err| err.to_string())?;
    let mut indexer = Indexer::new(conn, Box::new(StubEmbedder), 1);
    indexer.index_vault(&root)?;

    let hits = indexer.search_fts("indexer nasıl çalışıyor?", 5)?;
    assert!(hits.iter().any(|hit| hit.note_path.ends_with("indexer.md")));
    let hits = indexer.search_fts("auth-flow", 5)?;
    assert!(hits.iter().any(|hit| hit.note_path.ends_with("indexer.md")));

    fs::remove_dir_all(&root).map_err(|err| err.to_string())?;
    Ok(())
}

fn chunk(conn: &db::Connection, path: &str, text: &str) -> db::Result<i64> {
    db::upsert_note(conn, path, path, 1, path, None)?;
    db::insert_chunk(conn, path, None, 1, path, 0, &format!("{path}:0:v1"), text)
}
