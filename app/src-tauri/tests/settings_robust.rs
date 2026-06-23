use app_lib::settings::{self, AdvancedRetrievalSettings, LaneSettings, LocalGenSettings, Settings};
use std::fs;

#[test]
fn corrupt_settings_return_defaults_and_preserve_with_backup() {
    let root = test_dir("settings-corrupt");
    let path = root.join("settings.json");
    fs::create_dir_all(&root).expect("test settings dir should be created");
    fs::write(&path, "{not valid json").expect("corrupt settings should be written");

    let loaded = settings::load_from(&path);

    // audit #13: bellekte default dön, AMA bozuk-ama-kurtarılabilir dosyayı EZME; .bak yedekle.
    assert_eq!(loaded, Settings::default());
    let original = fs::read_to_string(&path).expect("original corrupt file should be preserved");
    assert_eq!(original, "{not valid json", "bozuk dosya ezilmemeli");
    let backup =
        fs::read_to_string(path.with_extension("json.bak")).expect(".bak yedeği yazılmalı");
    assert_eq!(backup, "{not valid json", ".bak orijinal içeriği taşımalı");

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
        semantic_search: false,
        advanced_retrieval: AdvancedRetrievalSettings::default(),
        ..Settings::default()
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

#[test]
fn advanced_retrieval_k_values_clamped() {
    // audit #2: bozuk/dev k değerleri normalized()'da makul aralığa sıkışmalı (OOM/var-limit koruması).
    let root = test_dir("settings-clamp");
    let path = root.join("settings.json");
    fs::create_dir_all(&root).expect("dir");
    fs::write(
        &path,
        r#"{"advanced_retrieval":{"enabled":true,"candidate_k":1000000000,"final_k":99999,"seed_k":99999,"graph_hops":99,"graph_neighbors_per_seed":99999,"semantic_cache_threshold":9999}}"#,
    )
    .expect("write");

    let adv = settings::load_from(&path).advanced_retrieval;
    assert!(adv.candidate_k <= 512, "candidate_k={}", adv.candidate_k);
    assert!(adv.final_k <= 64, "final_k={}", adv.final_k);
    assert!(adv.seed_k <= 128, "seed_k={}", adv.seed_k);
    assert!(adv.graph_hops <= 4, "graph_hops={}", adv.graph_hops);
    assert!(adv.graph_neighbors_per_seed <= 64);
    assert!(adv.semantic_cache_threshold <= 100);

    fs::remove_dir_all(&root).expect("cleanup");
}

fn test_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("aura-{name}-{}", std::process::id()))
}
