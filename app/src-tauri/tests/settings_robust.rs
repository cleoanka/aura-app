use app_lib::settings::{self, LaneSettings, LocalGenSettings, Settings};
use std::fs;

#[test]
fn corrupt_settings_return_defaults_and_rewrite() {
    let root = test_dir("settings-corrupt");
    let path = root.join("settings.json");
    fs::create_dir_all(&root).expect("test settings dir should be created");
    fs::write(&path, "{not valid json").expect("corrupt settings should be written");

    let loaded = settings::load_from(&path);

    assert_eq!(loaded, Settings::default());
    let rewritten = fs::read_to_string(&path).expect("settings should be rewritten");
    assert!(serde_json::from_str::<Settings>(&rewritten).is_ok());

    fs::remove_dir_all(&root).expect("test settings dir should be removed");
}

#[test]
fn partial_settings_fill_defaults_and_round_trip() {
    let root = test_dir("settings-partial");
    let path = root.join("settings.json");
    fs::create_dir_all(&root).expect("test settings dir should be created");
    fs::write(
        &path,
        r#"{
          "vault_roots": ["/tmp/vault"],
          "default_mode": "aura",
          "lanes": { "fast_enabled": false },
          "consensus_enabled": true,
          "cache_mode": "semantic",
          "local_gen": { "provider": "ollama", "model": "nomic" }
        }"#,
    )
    .expect("partial settings should be written");

    let loaded = settings::load_from(&path);
    let expected = Settings {
        vault_roots: vec!["/tmp/vault".to_string()],
        default_mode: "aura".to_string(),
        lanes: LaneSettings {
            fast_enabled: false,
            deep_enabled: true,
            lane0_enabled: false,
        },
        consensus_enabled: true,
        cache_mode: "semantic".to_string(),
        theme: "dark".to_string(),
        local_gen: LocalGenSettings {
            provider: "ollama".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            model: "nomic".to_string(),
        },
    };
    assert_eq!(loaded, expected);

    settings::save_to(&path, &loaded).expect("settings should save");
    let round_tripped = settings::load_from(&path);
    assert_eq!(round_tripped, expected);

    fs::remove_dir_all(&root).expect("test settings dir should be removed");
}

#[test]
fn wrong_field_types_default_per_field() {
    let root = test_dir("settings-wrong-types");
    let path = root.join("settings.json");
    fs::create_dir_all(&root).expect("test settings dir should be created");
    fs::write(
        &path,
        r#"{
          "vault_roots": "not a list",
          "default_mode": "aura",
          "lanes": { "fast_enabled": "bad", "deep_enabled": false, "lane0_enabled": true },
          "consensus_enabled": "bad",
          "cache_mode": 12,
          "theme": "light",
          "local_gen": { "provider": "mlx", "ollama_url": 22, "model": "local" }
        }"#,
    )
    .expect("wrong-type settings should be written");

    let loaded = settings::load_from(&path);

    assert!(loaded.vault_roots.is_empty());
    assert_eq!(loaded.default_mode, "aura");
    assert!(loaded.lanes.fast_enabled);
    assert!(!loaded.lanes.deep_enabled);
    assert!(loaded.lanes.lane0_enabled);
    assert!(!loaded.consensus_enabled);
    assert_eq!(loaded.cache_mode, "exact");
    assert_eq!(loaded.theme, "light");
    assert_eq!(loaded.local_gen.provider, "mlx");
    assert_eq!(loaded.local_gen.ollama_url, "http://localhost:11434");
    assert_eq!(loaded.local_gen.model, "local");

    fs::remove_dir_all(&root).expect("test settings dir should be removed");
}

fn test_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("aura-{name}-{}", std::process::id()))
}
