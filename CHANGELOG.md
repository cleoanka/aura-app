# Changelog

Bu projenin tüm dikkate değer değişiklikleri burada belgelenir.
Format [Keep a Changelog](https://keepachangelog.com/), sürümleme [SemVer](https://semver.org/).

## [Unreleased] — branch `a1`
### Added
- **Workspace (repo) semantiği**: aynı anda TEK aktif çalışma klasörü — dosya listesi,
  arama (FTS/hybrid), graf ve Ask bağlamı aktif köke daralır; yeni klasör seçmek eskisini
  anında listeden düşürür. MRU **switcher** (son çalışma alanları) + **unut** (workspace'in
  DB'deki tüm izleri temizlenir). Yeni komutlar: `get_workspace` / `set_active_workspace` /
  `forget_workspace`. Regresyon kalkanı: `tests/workspace.rs` (+ guard: pasif kök reddi).
- Workspace switcher görseli + animasyonlu GIF (`docs/assets/workspace-switch.gif`, üretici `_gen_switcher.py`).
- `scripts/soul_check.py` — anayasa (gizlilik / "modele doğrudan konuşma yok" /
  shell-injection yok / ağır özellikler default-OFF / cache doğruluğu) otomatik denetimi; CI'ye bağlandı.
- Otonom geliştirme altyapısı: `DEV_JOURNAL.md`, `IDEAS.md`, `BENCHMARKS.md`, `RESEARCH/`.
- Standart repo dosyaları: `CONTRIBUTING.md`, `CITATION.cff`, bu `CHANGELOG.md`.
- `docs/philosophy.md`, `docs/simple.md` (sıfır-jargon), `docs/glossary.md`; README'den linklendi.
- README'de gerçek **CI status rozeti**.
- `IndexStats.elapsed_ms` (indeksleme süresi) — backend + VaultExplorer'da gösterim; `pruned` da gösteriliyor.
- SearchPanel arama-gecikmesi göstergesi ("N sonuç · X ms", D37).
- **Semantic-cache** (opt-in, default OFF): anlamca-benzer soruları cosine≥threshold ile yakalar; İKİ kapı (threshold + dep-hash recheck) anayasa Madde 9'u (sıfır yanlış-cevap) korur. Gerçek-e5 eval'i FP=0 @0.96 ile kanıtlı (`tests/semantic_cache_eval.rs`).
- **sqlite-vec ANN** (D30-31): veri katmanı **bundled sqlite**'a (rusqlite) taşındı; `vec_search` artık vec0 KNN (cosine) — büyük vault'ta ölçekli ANN, brute-force fallback. Davranış-eşdeğer (tüm testler).
- **Stress test** (eşzamanlı reindex↔ask) + db/ai/markdown/cache/workspace testleri → **cargo test 63 → 95**.
- `.editorconfig` (UTF-8 / LF / final-newline / trim-ws; rustfmt-uyumlu 100-kol).
- `app/.env.example` — `.gitignore`'daki `!.env.example` askıda referansını çözer; app'in gerçekten okuduğu env override'larını (`TAURI_DEV_HOST`, `AURA_RUNS_DIR_HOME`) belgeler.
- `docs/history/` — büyük planlama/build-log dokümanları (`ultraplan-FINAL.md`, `plan-v2.1.md`, `PROGRESS.md`, `faz0/`, `vector-optimization-notes.md`) arşivlendi; kök/`docs/` sadeleşti.
- Türkçe deep-dive dokümanlarına kısa İngilizce özet (abstract) eklendi.
### Changed
- **CI sertleşti**: `cargo fmt --check` + `cargo clippy --all-targets -D warnings` + frontend `vitest` kapıları eklendi (workflow `a1`'de de çalışır); tüm kaynak rustfmt kanonik biçimine getirildi.
- `app/README.md` stok Tauri şablonundan gerçek geliştirici rehberine dönüştürüldü.
- BYOK anahtar doğrulaması: app `validate_key` + CLI `aura key set` (tek-token; boşluk/satır içeren yanlış-yapıştırma reddedilir).
### Performance
- **Async komut katmanı**: `index_vault` / `search_hybrid` / `search_fts` / `forget_workspace` /
  `agent_detect` / `agent_install` / `ollama_status` artık async + `spawn_blocking` — eskiden
  SYNC komut ana thread'de Indexer kilidini beklerken TÜM UI donuyordu (indeksleme sırasında
  arama = tam donma). `pick_vault_folder`'daki bloklayan `recv()` de oneshot+`await`'e geçti.
- **Açılış**: yalnız AKTİF workspace reindekslenir (eskiden TÜM geçmiş kökler taranıyordu);
  candle/e5 modeli **lazy** yüklenir (pencere ilk embed'e kadar beklemez); VaultExplorer
  listesine `content-visibility` (D33).
- **Sorgu embedding'i** artık 512 token'a pad edilmiyor → kısa sorguda ~10-30× daha az BERT hesabı.
- SQLite `synchronous=NORMAL` (WAL ile güvenli) + `embed_pending` batch'i tek transaction —
  yazım/fsync maliyeti düştü. `vec_ann` için açılışta GC + silmede senkron temizlik.
- Frontend: `key=dataVersion` tam-remount deseni kaldırıldı — VaultExplorer/GraphView prop ile
  tazelenir (scroll/zoom/pan/simülasyon state'i korunur); GraphView'da her engine-stop'ta
  viewport'u sıfırlayan `zoomToFit` tek-seferlik ilk-fit'e indirildi; `onNodeDrag` her-frame
  reheat → `onNodeDragEnd` tek reheat. Release profili: `lto=thin` + `codegen-units=1` + `strip`.
### Fixed
- **Cache doğruluğu (kritik)**: not/chunk silindiğinde `cache_deps` CASCADE ile yok olup
  dep'siz kalan cache girdisi "geçerli" sayılıyordu → BAYAT cevap dönebiliyordu. Artık silme
  öncesi bağımlı cache girdileri düşürülür; regresyon testleri `tests/cache_invalidation.rs`'de.
- **file_id yol-tabanlı** yapıldı: atomic-save yapan editörlerde (temp+rename → yeni inode)
  her kayıt tüm `chunk_stable_id`'leri değiştirip TAM re-embed + cache silme tetikliyordu.
- Lane ayarları IPC uyumsuzluğu: frontend `fast/deep/lane0` yazarken backend `*_enabled`
  bekliyordu → toggle'lar sessiz no-op'tu.
- Tek okunamayan dizin veya NUL-baytlı dosya TÜM indekslemeyi düşürüyordu → artık o girdi atlanır.
- Startup'ta DB açılamazsa panic yerine kullanıcıya nedenli hata dialogu; startup reindex
  hatası UI'a `index-error` event'iyle taşınır ("index-updated" yalnız başarıda).
- `capture_login_env` (`zsh -lc env`) artık 10 sn timeout'lu — bozuk dotfile uygulamayı asamaz.
- `aura doctor` JSON'u dotfile gürültüsüne dayanıklı (ilk `{`…son `}` bloğu parse edilir).
- Consensus: toplam zaman aşımı ayardan türetilir (sabit 420s, ayarın izin verdiği 600s'lik
  ajanı kesiyordu); başarısız/başlatılamayan ajanlar stderr kuyruğuyla UI'da raporlanır;
  ilerleme sayacı yalnız gerçekten başlatılan ajanları sayar; zaman aşımı `timeout`
  taksonomisiyle etiketlenir (eskiden `network`).
- Lane0 (Ollama) isteği job registry'ye kaydedilir → **Stop butonu çalışır** (eskiden 300 sn iptal edilemezdi).
- `deep_query` kelime-sınırlı eşleşmeye geçti — "planet"/"airplane" gibi kelimeler 'plan'
  substring'i yüzünden pahalı deep lane'e yönlenmiyor.
- PTY: kill sonrası `wait()` (zombi süreç birikmiyor); PtyLogin'de agent değişiminde ikinci
  oturumun kapanmaması (ref-guard) düzeltildi.
- AI cevabındaki `http(s)` linkleri sistem tarayıcısında açılır (WebView'i uygulamadan koparmaz).
- Chat composer WKWebView'de büyümüyordu (`field-sizing` desteklenmiyor) → JS autosize;
  hızlı stream'de autoscroll'un kalıcı susması düzeltildi (dipte-kalma scroll-event'te izlenir).
- `get_graph` DB hatası boş graf yerine gerçek hata döndürür; commit hatasında read-bağlantısı
  açık transaction'da bırakılmaz.
- i18n/UX: `html lang` açılışta set edilir; şablon `<title>` → "AURA"; light temada açılış
  FOUC'u giderildi (localStorage ön-uygulama); ErrorBoundary/LiveActivity/NoteEditor/
  VaultExplorer'daki hardcoded Türkçe metinler i18n'e bağlandı; saniye sayacının
  ekran-okuyucu spam'i giderildi (aria-live daraltıldı); graf legend'ine eksik
  `config`/`external` tipleri eklendi; Aura-Mode'da consensus, ayar kapalıyken listeden kalkar.

## [0.2.0] — 2026-06-23
### Added
- **BYOK** — kendi Anthropic API anahtarınla çalışma (app + `aura key` CLI; `~/.aura/anthropic_api_key`, 0600; default OFF).
- `.gitignore`-duyarlı indeksleme (denylist + vault'un kendi `.gitignore`'u).
- `tests/cache_invalidation.rs` — cache'in dosya-hash'leriyle senkron geçersizleşmesini kanıtlayan regresyon testi.
### Fixed
- CI: `.dmg` paketleme headless runner'da Finder gerektirdiği için patlıyordu → CI artık `--bundles app`.
- gemini→agy yeniden adlandırmasından kalma 3 stale test (`settings_robust`, `consensus_degrade`, `pty_argv`) düzeltildi → tam paket yeşil (63 test).
### Security
- Release binary'leri `--remap-path-prefix` ile derlenir → gömülü kişisel yol/kullanıcı adı yok.

## [0.1.0] — 2026-06-23
### Added
- İlk public sürüm: Tauri v2 + Rust + React/TS ikinci-beyin; Agent Manager, hibrit arama, knowledge graph, Ask (cache→retrieve→lane→stream), aura-mode, consensus, Lane 0.
