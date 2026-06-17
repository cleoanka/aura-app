use app_lib::db;

#[test]
fn vector_fallback_and_fts5_work() -> db::Result<()> {
    let conn = db::open_in_memory()?;

    db::upsert_note(
        &conn,
        "alpha.md",
        "file-alpha",
        1,
        "hash-alpha",
        Some("Alpha"),
    )?;
    db::upsert_note(&conn, "beta.md", "file-beta", 2, "hash-beta", Some("Beta"))?;
    db::upsert_note(
        &conn,
        "gamma.md",
        "file-gamma",
        3,
        "hash-gamma",
        Some("Gamma"),
    )?;

    let alpha_chunk = db::insert_chunk(
        &conn,
        "alpha.md",
        None,
        1,
        "Alpha",
        0,
        "file-alpha:Alpha:0:v1",
        "The orchard contains a single pomegranate tree.",
    )?;
    let beta_chunk = db::insert_chunk(
        &conn,
        "beta.md",
        None,
        1,
        "Beta",
        0,
        "file-beta:Beta:0:v1",
        "A project note about indexing embeddings.",
    )?;
    let gamma_chunk = db::insert_chunk(
        &conn,
        "gamma.md",
        None,
        1,
        "Gamma",
        0,
        "file-gamma:Gamma:0:v1",
        "Unrelated archive material.",
    )?;

    let query = embedding(0.20);
    db::insert_embedding(&conn, alpha_chunk, &embedding(0.20))?;
    db::insert_embedding(&conn, beta_chunk, &embedding(0.21))?;
    db::insert_embedding(&conn, gamma_chunk, &embedding(-0.80))?;

    let vector_matches = db::vec_search(&conn, &query, 3)?;
    assert_eq!(vector_matches[0].0, alpha_chunk);
    assert_eq!(vector_matches[1].0, beta_chunk);
    assert_eq!(vector_matches[2].0, gamma_chunk);

    let fts_matches = db::fts_search(&conn, "pomegranate", 5)?;
    assert_eq!(fts_matches[0].0, alpha_chunk);

    let schema_version = db::meta_value(&conn, "schema_version")?;
    assert_eq!(schema_version.as_deref(), Some("1"));

    Ok(())
}

fn embedding(seed: f32) -> [f32; db::EMBEDDING_DIM] {
    let mut values = [0.0; db::EMBEDDING_DIM];
    for (index, value) in values.iter_mut().enumerate() {
        let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
        *value = seed + sign * (index as f32 * 0.0001);
    }
    values
}
