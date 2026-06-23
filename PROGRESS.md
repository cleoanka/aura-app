# AURA Desktop — Build Progress

Otonom build (Opus 4.8 orkestratör + Codex implementer + Gemini verify). Ultraplan: `docs/ultraplan-FINAL.md`.

## Faz 0 — Platform ölüm-kalım smoke ✅ GO
- T0.0 ortam: Rust 1.93, Node 24.15, codesign, brew ✓; Tauri CLI yok (npm-local kullanılacak).
- T0.1 token: claude=**keychain**, gemini=dosya, codex=dosya.
- T0.2 local-hardened codesign: **PASS** (sadece `inherit`, unsigned-mem=0).
- T0.3 notary: ATLANDI (Apple ID yok) — bloker değil, T4.6'ya ertelendi.
- T0.4 quarantine-spawn: KISMİ PASS (bundle child aura çalıştı).
- T0.5 cold-start: ek-median **30ms** « 1.5s → **per-job spawn, daemon YOK**.
- T0.6 env-resolver: zsh -lc & -c ikisi de binary buluyor → nice-to-have.
- T0.7 **OPUS GO** ✅. Entitlement: NON-SANDBOXED Developer ID + hardened + inherit.
- Detay: `docs/faz0/bulgular.md`.

## Faz 1 — Agent Manager + sözleşmeler + vault (DEVAM EDİYOR)
- [ ] T1.1 Tauri v2 + plugin-shell + ACL iskeleti + Playwright
- [x] T1.2 aura `--prompt-file/--context/--json-events` + `doctor --json` ✓ (json-events start→chunk→done; doctor --json sözleşme GEÇTİ)
- [x] T1.3 doctor sözleşmesi contracts/ ✓ (Rust testi AM işinde)
- [x] T1.4 env_resolver + ErrorTaxonomy (Rust) ✓
- [x] T1.5 detect/install/doctor + Tauri commands + **React UI kartları** ✓ (npm build PASS); PTY login SIRADA
- [ ] T1.6 vault folder-picker + settings persist
- [ ] T1.7 gerçek .app re-smoke

## Faz 2 — Index + hibrit arama (DEVAM)
- [x] 2a veri katmanı: SQLite (sistem libsqlite3 FFI) + FTS5 gerçek + vektör brute-force fallback + cache/cache_deps/meta şema + db_smoke test PASS
  - NOT: codex sandbox ağsız → rusqlite/sqlite-vec indiremedi; FFI ile sistem sqlite kullanıldı. Tech-debt: Faz 4'te rusqlite+sqlite-vec'e geçiş (deps'i ben ekleyeceğim).
- [x] 2b indexer (markdown/wikilink/hierarchical chunk/petgraph graph/incremental) + Embedder trait (StubEmbedder; candle Faz2c+) ✓ cargo test PASS
- [x] 2c hibrit arama (FTS5+vektör RRF) + settings(robust, consensus/lane0 default OFF) + vault picker + list_notes ✓ cargo test PASS (ben build ettim; codex offline)
- [x] 2d workspace UI + Ask AI paneli (streaming) ✓ npm build PASS

## ⚙️ SÜREÇ NOTU: codex ağsız → Rust deps'i Claude (Bash, ağlı) `cargo add` ile ekler, codex kodu yazar.
## Faz 3 — AI akışı (DEVAM)
- [x] exec.rs per-job aura runner (json-events→Channel) + pgid cancel + temp prompt/context 0600
- [x] commands/ai.rs ask (exact cache→retrieve→lane→stream) + cancel_job; vault.rs read/write guard (traversal blok)
- [x] db cache get/put/deps; cache_key + vault_guard testleri PASS
- [x] Lane 0 (yerel üretim Ollama, ureq, ask'e entegre, default off) ✓
- [ ] Consensus modu (3 AI→sentez, default off)
## 🎉 MILESTONE: çalışan .app + .dmg üretildi (release, arm64, ad-hoc imzalı, açılıyor/çökmüyor)

## Faz 4 — Cila + paketleme (DEVAM)
- [x] productName "AURA Desktop" + pencere 1280×820 Overlay titlebar
- [ ] CodeMirror editör + react-force-graph gerçek graf
- [x] aura-mode (plan/review/fix/ship app-içi, fix=dry-run güvenli) ✓
- [x] Lane 0 (Ollama ureq, default off) ✓
- [x] Consensus (3 AI paralel→claude sentezi, graceful degrade, default off) ✓ test PASS
- [x] PTY login paneli (xterm + portable-pty, app-içi OAuth) ✓ pty_argv test PASS
- [x] uyarılar temizlendi (0 warning, 14 test binary OK)
- [x] **FINAL: 'AURA Desktop.app' (14M) + .dmg (5.6M) build + smoke (açılıyor/çökmüyor) ✓**
- [x] gerçek candle e5 embedding: `--features candle` DERLENİYOR ✓ (default StubEmbedder korunur); runtime model-download ilk kullanımda
- [ ] notarize: kullanıcı Apple Developer ID gerekir (T4.6) — dev build lokal çalışır

## ✅ TAM TEST TARAMASI (hepsi GEÇTİ)
1. Rust `cargo test` (default): **23 test PASS** + 1 ignored (db, indexer, hybrid RRF, settings-robust, cache_key, vault_guard, pty_argv, modes_argv, consensus_prompt, doctor_contract).
2. `cargo build --features candle`: **DERLENİYOR** (gerçek e5 embedding opt-in); default build StubEmbedder + 0 uyarı.
3. Frontend `npm run build` (tsc+vite): **PASS** (1105 modül, 0 tip hatası).
4. aura engine: `--json-events` start→chunk→done **PASS**; `doctor --json` sözleşme **PASS** (claude=keychain).
5. `npm run tauri build`: **AURA Desktop.app (14M) + .dmg (6.6M)** üretildi.
6. App smoke: **açılıyor, çökmüyor** ✓.

Headless yapılamayan (kullanıcı): canlı GUI tıklama akışı (vault→index→ara→Ask), notarization (Apple ID).

## 🔧 Kullanıcı geri bildirimi düzeltmeleri (UI/i18n)
- Vault picker çökmesi → async + non-blocking dialog (ana-thread deadlock fix).
- Titlebar overlap → Overlay kaldırıldı.
- Çirkin tek-harf rail ikonları → gerçek SVG ikon seti + marka simgesi.
- 'Vault Seç' jargonu → 'Not Klasörü Aç' (gemini EN/TR string tablosu).
- **EN/TR dil desteği**: i18n (97 anahtar), rail'de canlı toggle, 10 panel t()'ye geçti (paralel claude workflow).
- Ekranlar: `docs/assets/` — kişisel veri içermeyen sentetik demo görselleri (banner + workspace + knowledge graph; SVG kaynak + üretici script + PNG render). README'de gömülü.

## 🔧 v2 oturumu (proje second-brain + model yönetimi + bug fix)
- Code-aware indexer: TÜM dosya tipleri + diller-arası graf (py/c/rust/js/ts/go import + [[wikilink]] + generic .o mention); db v2 links tablosu.
- Vektör optimizasyonu: e5 passage/query prefix, Device::Cpu (Accelerate), L2-normalized + partial top-k, chunk content_hash incremental, code-aware chunking.
- candle gerçek embedding DEFAULT (opt-in değil); başlangıç-güvenli: model cache'liyse candle, yoksa Stub+FTS5 (İNDİRME YOK).
- Model Manager: embedding indir/durum (force_prepare_candle) + Ollama durum/pull + cloud agent install/PTY-login — 3 net bölüm.
- Consensus graceful-degrade: tek ajan→direkt, sentez yoksa başlıklı birleştirme, sadece-claude çalışır.
- GraphView Obsidian-overhaul: kind/dosya-tipi renkleri, degree-boyut, kontrol paneli (node boyutu/link mesafesi/etiket/fit/legend).
- i18n: 'Proje' reframe (Not→Proje/Project) + 30 model/graph anahtarı; EN/TR.
- **KRİTİK fix:** başlangıçta candle model indirme pencereyi açmıyordu (hang) → default_embedder artık indirmez.
- Doğrulama: cargo build + 17 test PASS; npm build temiz; .app açılıyor (37 dosyalı proje indexlendi, multi-filetype çalışıyor); /Applications kurulu.

## 🔧 v3 oturumu (BYOK + sağlamlaştırma + sürüm 0.2.0)
- **API key (BYOK):** `~/.aura/anthropic_api_key` (0600) tek paylaşılan kaynak — hem app hem CLI okur. App: `apikey.rs` + `commands/apikey.rs` (set/clear/status), Settings `api_key_enabled` (default OFF), exec.rs + consensus.rs çocuk sürece `ANTHROPIC_API_KEY` enjekte eder. UI: ModelManager 4. bölüm (password input + maskeli durum + enable toggle), i18n EN/TR. CLI: `aura key set|status|clear` (v0.5.0), `_apply_api_key()` çocuklara aktarır, doctor durum satırı.
- **`.gitignore` desteği:** indexer denylist'e EK olarak vault kök `.gitignore`'undaki basit dizin/dosya adlarını da dışlar (`gitignore_names`/`is_ignored_path`) → kara-delik klasör koruması güçlendi.
- **Cache invalidation (doğrulandı + test):** zaten doğru (retrieval-fingerprint cache_key'de + cache_get_valid dep content-hash kontrolü); yeni `tests/cache_invalidation.rs` (3 test) bunu kanıtlıyor. ai.rs'e netleştirici yorum.
- **Pre-existing kırık testler düzeltildi:** gemini→agy rename'inden kalma `settings_robust` (eksik alanlar), `consensus_degrade` ("gemini"→"agy"), `pty_argv` ("gemini"→"agy"). Artık tam paket yeşil.
- **Shell-overhead:** zaten çözülmüş (env_resolver OnceLock — login shell oturumda 1 kez) → doküman netleştirildi.
- Sürüm **0.1.0 → 0.2.0** (tauri.conf + package.json + Cargo.toml).
- Doğrulama: `cargo test` **63 test / 27 suite PASS, 0 hata**; `npm run build` temiz (tsc 0 hata); CLI py_compile + `aura key` fonksiyonel test (0600 dosya, maskeli durum).
