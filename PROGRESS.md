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
- [ ] Lane 0 (yerel üretim Ollama/MLX) — settings'te var, exec entegrasyonu sırada
- [ ] Consensus modu (3 AI→sentez, default off)
## 🎉 MILESTONE: çalışan .app + .dmg üretildi (release, arm64, ad-hoc imzalı, açılıyor/çökmüyor)

## Faz 4 — Cila + paketleme (DEVAM)
- [x] productName "AURA Desktop" + pencere 1280×820 Overlay titlebar
- [ ] CodeMirror editör + react-force-graph gerçek graf
- [ ] aura-mode (plan/review/fix/ship app-içi)
- [ ] Consensus (3 AI→sentez, default off) + Lane 0 (Ollama/MLX)
- [ ] PTY login paneli
- [ ] notarize (kullanıcı Apple ID — T4.6)
