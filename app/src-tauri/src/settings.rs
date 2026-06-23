use serde::{Deserialize, Deserializer, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(
        default = "default_vault_roots",
        deserialize_with = "deserialize_vault_roots"
    )]
    pub vault_roots: Vec<String>,
    #[serde(
        default = "default_default_mode",
        deserialize_with = "deserialize_default_mode"
    )]
    pub default_mode: String,
    #[serde(default = "default_lanes", deserialize_with = "deserialize_lanes")]
    pub lanes: LaneSettings,
    #[serde(
        default = "default_consensus_enabled",
        deserialize_with = "deserialize_consensus_enabled"
    )]
    pub consensus_enabled: bool,
    #[serde(
        default = "default_cache_mode",
        deserialize_with = "deserialize_cache_mode"
    )]
    pub cache_mode: String,
    #[serde(default = "default_theme", deserialize_with = "deserialize_theme")]
    pub theme: String,
    #[serde(
        default = "default_local_gen",
        deserialize_with = "deserialize_local_gen"
    )]
    pub local_gen: LocalGenSettings,
    /// Yerel semantic embedding (candle e5). VARSAYILAN KAPALI: CPU yoğun olduğu için
    /// (965 chunk'ı tek tek embed eder). Kapalıyken hızlı FTS5 araması kullanılır.
    #[serde(default)]
    pub semantic_search: bool,
    #[serde(default = "default_advanced_retrieval")]
    pub advanced_retrieval: AdvancedRetrievalSettings,
    #[serde(default = "default_consensus_settings")]
    pub consensus: ConsensusSettings,
    /// BYOK: when true, the stored Anthropic API key (`~/.aura/anthropic_api_key`)
    /// is injected into spawned agents as `ANTHROPIC_API_KEY`. Default OFF →
    /// existing subscription / OAuth auth is used unchanged.
    #[serde(default)]
    pub api_key_enabled: bool,
}

// Consensus zamanlaması — kullanıcı uygulama içinden ayarlar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsensusSettings {
    /// İlk ajan yanıtladıktan sonra geç kalanlara tanınan ek süre (sn). 0 = grace yok (hepsini bekle).
    #[serde(default = "default_consensus_grace")]
    pub grace_secs: u32,
    /// Bir ajanın sert üst sınırı (sn). Aşılırsa o ajan düşürülür.
    #[serde(default = "default_consensus_agent_timeout")]
    pub agent_timeout_secs: u32,
}

fn default_consensus_grace() -> u32 {
    // 0 = grace kapalı (hepsini agent_timeout'a kadar bekle). VARSAYILAN: agy/Antigravity ~40s
    // sürüyor (claude/codex ~5s); grace>0 onu sessizce DÜŞÜRÜYORDU. 0 ile agy hep dahil; "kırk saat"
    // riski yok çünkü agent_timeout (90s) zaten sert sınır. Hız isteyen kullanıcı UI'dan grace verir.
    0
}
fn default_consensus_agent_timeout() -> u32 {
    90
}
fn default_consensus_settings() -> ConsensusSettings {
    ConsensusSettings {
        grace_secs: default_consensus_grace(),
        agent_timeout_secs: default_consensus_agent_timeout(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaneSettings {
    #[serde(
        default = "default_fast_enabled",
        deserialize_with = "deserialize_fast_enabled"
    )]
    pub fast_enabled: bool,
    #[serde(
        default = "default_deep_enabled",
        deserialize_with = "deserialize_deep_enabled"
    )]
    pub deep_enabled: bool,
    #[serde(
        default = "default_lane0_enabled",
        deserialize_with = "deserialize_lane0_enabled"
    )]
    pub lane0_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalGenSettings {
    #[serde(
        default = "default_provider",
        deserialize_with = "deserialize_provider"
    )]
    pub provider: String,
    #[serde(
        default = "default_ollama_url",
        deserialize_with = "deserialize_ollama_url"
    )]
    pub ollama_url: String,
    #[serde(default = "default_model", deserialize_with = "deserialize_model")]
    pub model: String,
}

// Gelişmiş agentic-retrieval katmanı. Hepsi master `enabled` ile geçitli.
// VARSAYILAN KAPALI: enabled=false iken Ask yolu birebir eski (legacy) davranışta.
// threshold yüzde olarak (u32) tutulur (Settings `Eq` türetiyor; f32 kıracaktı).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdvancedRetrievalSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub planner_enabled: bool,
    #[serde(default = "default_true")]
    pub graph_enabled: bool,
    #[serde(default = "default_true")]
    pub rerank_enabled: bool,
    #[serde(default = "default_true")]
    pub parent_pull_in_enabled: bool,
    #[serde(default)]
    pub semantic_cache_enabled: bool,
    #[serde(default = "default_sem_threshold")]
    pub semantic_cache_threshold: u32, // yüzde, 96 = 0.96
    #[serde(default = "default_seed_k")]
    pub seed_k: u32,
    #[serde(default = "default_candidate_k")]
    pub candidate_k: u32,
    #[serde(default = "default_final_k")]
    pub final_k: u32,
    #[serde(default = "default_graph_hops")]
    pub graph_hops: u32,
    #[serde(default = "default_graph_neighbors")]
    pub graph_neighbors_per_seed: u32,
    #[serde(default)]
    pub planner_model: String,
}

fn default_true() -> bool {
    true
}
fn default_sem_threshold() -> u32 {
    96
}
fn default_seed_k() -> u32 {
    12
}
fn default_candidate_k() -> u32 {
    48
}
fn default_final_k() -> u32 {
    8
}
fn default_graph_hops() -> u32 {
    2
}
fn default_graph_neighbors() -> u32 {
    6
}
fn default_advanced_retrieval() -> AdvancedRetrievalSettings {
    AdvancedRetrievalSettings::default()
}

impl Default for AdvancedRetrievalSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            planner_enabled: true,
            graph_enabled: true,
            rerank_enabled: true,
            parent_pull_in_enabled: true,
            semantic_cache_enabled: false,
            semantic_cache_threshold: default_sem_threshold(),
            seed_k: default_seed_k(),
            candidate_k: default_candidate_k(),
            final_k: default_final_k(),
            graph_hops: default_graph_hops(),
            graph_neighbors_per_seed: default_graph_neighbors(),
            planner_model: String::new(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            vault_roots: default_vault_roots(),
            default_mode: default_default_mode(),
            lanes: default_lanes(),
            consensus_enabled: default_consensus_enabled(),
            cache_mode: default_cache_mode(),
            theme: default_theme(),
            local_gen: default_local_gen(),
            semantic_search: false,
            advanced_retrieval: AdvancedRetrievalSettings::default(),
            consensus: default_consensus_settings(),
            api_key_enabled: false,
        }
    }
}

impl Default for LaneSettings {
    fn default() -> Self {
        Self {
            fast_enabled: default_fast_enabled(),
            deep_enabled: default_deep_enabled(),
            lane0_enabled: default_lane0_enabled(),
        }
    }
}

impl Default for LocalGenSettings {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            ollama_url: default_ollama_url(),
            model: default_model(),
        }
    }
}

pub fn settings_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    path.push("aura-desktop");
    path.push("settings.json");
    path
}

pub fn load() -> Settings {
    load_from(&settings_path())
}

pub fn load_from(path: &Path) -> Settings {
    match fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<Settings>(&content) {
            Ok(settings) => settings.normalized(),
            Err(_) => {
                // Bozuk ama kurtarılabilir config'i EZME (audit #13): .bak'a yedekle,
                // bellekte default dön. Dosya bir sonraki set_settings'e kadar korunur.
                let _ = fs::write(path.with_extension("json.bak"), &content);
                Settings::default()
            }
        },
        // Dosya yok/okunamıyor: bellekte default (ilk set_settings yazar) — clobber yok.
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) -> Result<(), String> {
    save_to(&settings_path(), settings)
}

pub fn save_to(path: &Path, settings: &Settings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create settings directory: {err}"))?;
    }

    let tmp_path = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("settings.json"),
        std::process::id()
    ));
    let result = write_settings_file(&tmp_path, &settings.normalized()).and_then(|()| {
        fs::rename(&tmp_path, path).map_err(|err| format!("failed to replace settings file: {err}"))
    });

    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    result
}

impl Settings {
    fn normalized(&self) -> Self {
        let mut settings = self.clone();
        if !matches!(settings.default_mode.as_str(), "ask" | "aura") {
            settings.default_mode = default_default_mode();
        }
        if !matches!(settings.cache_mode.as_str(), "off" | "exact" | "semantic") {
            settings.cache_mode = default_cache_mode();
        }
        if settings.theme.trim().is_empty() {
            settings.theme = default_theme();
        }
        // Bellek/DoS koruması (audit #2): advanced retrieval k-değerlerini makul aralığa sıkıştır
        // → bozuk config dev allocation (vec_search) / SQLite değişken limiti aşımı yapamaz.
        let adv = &mut settings.advanced_retrieval;
        adv.seed_k = adv.seed_k.clamp(1, 128);
        adv.candidate_k = adv.candidate_k.clamp(1, 512);
        adv.final_k = adv.final_k.clamp(1, 64);
        adv.graph_hops = adv.graph_hops.clamp(0, 4);
        adv.graph_neighbors_per_seed = adv.graph_neighbors_per_seed.clamp(0, 64);
        adv.semantic_cache_threshold = adv.semantic_cache_threshold.clamp(50, 100);
        // Consensus bekleme süreleri makul aralıkta (kullanıcı UI'dan girer).
        settings.consensus.grace_secs = settings.consensus.grace_secs.min(600);
        settings.consensus.agent_timeout_secs = settings.consensus.agent_timeout_secs.clamp(10, 600);
        settings.local_gen = settings.local_gen.normalized();
        settings
    }
}

impl LocalGenSettings {
    fn normalized(&self) -> Self {
        let mut settings = self.clone();
        if !matches!(settings.provider.as_str(), "none" | "ollama" | "mlx") {
            settings.provider = default_provider();
        }
        if settings.ollama_url.trim().is_empty() {
            settings.ollama_url = default_ollama_url();
        }
        settings
    }
}

fn write_settings_file(path: &Path, settings: &Settings) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|err| format!("failed to serialize settings: {err}"))?;
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options
        .open(path)
        .map_err(|err| format!("failed to write settings temp file: {err}"))?;
    file.write_all(&bytes)
        .map_err(|err| format!("failed to write settings: {err}"))?;
    file.write_all(b"\n")
        .map_err(|err| format!("failed to write settings newline: {err}"))?;
    file.sync_all()
        .map_err(|err| format!("failed to sync settings: {err}"))?;
    Ok(())
}

fn default_vault_roots() -> Vec<String> {
    Vec::new()
}

fn default_default_mode() -> String {
    "ask".to_string()
}

fn default_lanes() -> LaneSettings {
    LaneSettings::default()
}

fn default_consensus_enabled() -> bool {
    false
}

fn default_cache_mode() -> String {
    "exact".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_local_gen() -> LocalGenSettings {
    LocalGenSettings::default()
}

fn default_fast_enabled() -> bool {
    true
}

fn default_deep_enabled() -> bool {
    true
}

fn default_lane0_enabled() -> bool {
    false
}

fn default_provider() -> String {
    "none".to_string()
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_model() -> String {
    String::new()
}

macro_rules! deserialize_or_default {
    ($name:ident, $ty:ty, $default:expr) => {
        fn $name<'de, D>(deserializer: D) -> Result<$ty, D::Error>
        where
            D: Deserializer<'de>,
        {
            Ok(<$ty>::deserialize(deserializer).unwrap_or_else(|_| $default()))
        }
    };
}

deserialize_or_default!(deserialize_vault_roots, Vec<String>, default_vault_roots);
deserialize_or_default!(deserialize_default_mode, String, default_default_mode);
deserialize_or_default!(deserialize_lanes, LaneSettings, default_lanes);
deserialize_or_default!(
    deserialize_consensus_enabled,
    bool,
    default_consensus_enabled
);
deserialize_or_default!(deserialize_cache_mode, String, default_cache_mode);
deserialize_or_default!(deserialize_theme, String, default_theme);
deserialize_or_default!(deserialize_local_gen, LocalGenSettings, default_local_gen);
deserialize_or_default!(deserialize_fast_enabled, bool, default_fast_enabled);
deserialize_or_default!(deserialize_deep_enabled, bool, default_deep_enabled);
deserialize_or_default!(deserialize_lane0_enabled, bool, default_lane0_enabled);
deserialize_or_default!(deserialize_provider, String, default_provider);
deserialize_or_default!(deserialize_ollama_url, String, default_ollama_url);
deserialize_or_default!(deserialize_model, String, default_model);
