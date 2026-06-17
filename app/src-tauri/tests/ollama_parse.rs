use app_lib::lane0::parse_ollama_tags_json;

#[test]
fn parses_ollama_tag_names() {
    let json = r#"{
        "models": [
            {"name": "llama3.2:latest"},
            {"name": "qwen2.5:7b"}
        ]
    }"#;

    assert_eq!(
        parse_ollama_tags_json(json),
        vec!["llama3.2:latest".to_string(), "qwen2.5:7b".to_string()]
    );
}

#[test]
fn malformed_ollama_tags_return_empty_list() {
    assert!(parse_ollama_tags_json("{not json").is_empty());
}
